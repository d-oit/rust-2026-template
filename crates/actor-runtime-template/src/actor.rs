//! Actor implementation patterns.

use super::{ActorError, ActorState, Lifecycle};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// Actor message envelope.
pub type Envelope<S> = mpsc::Sender<ActorMessage<S>>;

/// Actor message with state.
#[derive(Debug)]
pub enum ActorMessage<S: ActorState> {
    /// Initialize actor with initial state.
    Init(S),
    /// Process a work item.
    Process(String),
    /// Get current state.
    GetState {
        /// Channel to send the state on.
        respond_to: mpsc::Sender<S>,
    },
    /// Stop the actor.
    Stop,
    /// Health check ping.
    Ping {
        /// Channel to send the response on.
        respond_to: mpsc::Sender<()>,
    },
}

/// Actor handle for external control.
pub struct ActorHandle<S: ActorState> {
    tx: mpsc::Sender<ActorMessage<S>>,
    state: S,
}

impl<S: ActorState> ActorHandle<S> {
    /// Create a new actor handle.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "const not possible across MSRV/generic bounds"
    )]
    pub fn new(tx: mpsc::Sender<ActorMessage<S>>, state: S) -> Self {
        Self { tx, state }
    }

    /// Send a message to the actor.
    pub async fn send(&self, msg: ActorMessage<S>) -> Result<(), ActorError> {
        self.tx.send(msg).await.map_err(|_| ActorError::MailboxFull)
    }

    /// Get a copy of the current state.
    pub fn state(&self) -> S {
        self.state.clone()
    }
}

/// Actor runtime with message processing.
pub struct Actor<S: ActorState> {
    state: S,
    restart_strategy: super::RestartStrategy,
}

/// Upper bound (in escaped characters) for a work payload rendered into a log line.
const MAX_LOGGED_LEN: usize = 256;

/// Escape control/Bidi/format characters and bound the length of a payload for logging.
///
/// Runs as a single bounded pass: a source character is emitted only when its whole escape
/// sequence fits within [`MAX_LOGGED_LEN`], so the payload is never materialised in full and a
/// truncation suffix can never split an escape sequence mid-way.
///
/// Returns the escaped string, or the escaped prefix followed by `... [truncated]`.
fn sanitize_log_payload(work: &str) -> String {
    let bytes = work.as_bytes();
    let mut i = 0;

    // Fast path: SWAR (SIMD Within A Register) check 8-byte chunks simultaneously
    // for printable ASCII values (0x20-0x7E) excluding '\\', '"', and '\''.
    while i + 8 <= bytes.len() {
        let mut arr = [0u8; 8];
        arr.copy_from_slice(&bytes[i..i + 8]);
        let chunk = u64::from_ne_bytes(arr);

        let has_low = (chunk.wrapping_sub(0x2020_2020_2020_2020) & !chunk) & 0x8080_8080_8080_8080;
        let has_high = chunk & 0x8080_8080_8080_8080;
        let y = chunk ^ 0x7F7F_7F7F_7F7F_7F7F;
        let zero_check = (y.wrapping_sub(0x0101_0101_0101_0101) & !y) & 0x8080_8080_8080_8080;

        let contains_byte = |b: u8| {
            let mask = u64::from_ne_bytes([b; 8]);
            let x = chunk ^ mask;
            (x.wrapping_sub(0x0101_0101_0101_0101) & !x) & 0x8080_8080_8080_8080
        };

        if (has_low
            | has_high
            | zero_check
            | contains_byte(b'\\')
            | contains_byte(b'"')
            | contains_byte(b'\''))
            != 0
        {
            break;
        }
        i += 8;
    }

    // Scalar fallback loop for remaining bytes
    while i < bytes.len() {
        let b = bytes[i];
        if !(0x20..=0x7E).contains(&b) || b == b'\\' || b == b'"' || b == b'\'' {
            break;
        }
        i += 1;
    }

    if i == bytes.len() {
        if bytes.len() <= MAX_LOGGED_LEN {
            return work.to_string();
        } else {
            let mut out = String::with_capacity(MAX_LOGGED_LEN + "... [truncated]".len());
            out.push_str(&work[..MAX_LOGGED_LEN]);
            out.push_str("... [truncated]");
            return out;
        }
    }

    if i >= MAX_LOGGED_LEN {
        let mut out = String::with_capacity(MAX_LOGGED_LEN + "... [truncated]".len());
        out.push_str(&work[..MAX_LOGGED_LEN]);
        out.push_str("... [truncated]");
        return out;
    }

    // Slow path: some characters need escaping before MAX_LOGGED_LEN.
    // Cache `escape_debug()` into a stack buffer to avoid double-iteration over escape sequences.
    let mut out = String::with_capacity(work.len().min(MAX_LOGGED_LEN) + 16);
    out.push_str(&work[..i]);
    let mut used = i;

    for ch in work[i..].chars() {
        let mut esc_buf = ['\0'; 12];
        let mut esc_len = 0;
        for esc_ch in ch.escape_debug() {
            esc_buf[esc_len] = esc_ch;
            esc_len += 1;
        }

        if used + esc_len > MAX_LOGGED_LEN {
            out.push_str("... [truncated]");
            return out;
        }

        out.extend(&esc_buf[..esc_len]);
        used += esc_len;
    }

    out
}

impl<S: ActorState> Actor<S> {
    /// Create a new actor with initial state.
    pub fn new(state: S) -> Self {
        Self {
            state,
            restart_strategy: Default::default(),
        }
    }

    /// Set the restart strategy.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "const not possible across MSRV/generic bounds"
    )]
    pub fn with_strategy(mut self, strategy: super::RestartStrategy) -> Self {
        self.restart_strategy = strategy;
        self
    }

    /// Handle a message and update state.
    pub async fn receive(&mut self, msg: ActorMessage<S>) -> Result<Lifecycle, ActorError> {
        match msg {
            ActorMessage::Init(new_state) => {
                self.state = new_state;
                self.state.validate()?;
                info!("Actor initialized");
                Ok(Lifecycle::Started)
            }
            ActorMessage::Process(work) => {
                // Security: Escape control/Bidi characters and bound the logged length so a
                // payload can neither forge a log record nor fill the log (see
                // `sanitize_log_payload`).
                debug!("Processing: {}", sanitize_log_payload(&work));

                Ok(Lifecycle::Started)
            }
            ActorMessage::GetState { respond_to } => {
                let _ = respond_to.send(self.state.clone()).await;
                Ok(Lifecycle::Started)
            }
            ActorMessage::Stop => {
                info!("Actor stopped");
                Ok(Lifecycle::Stopped)
            }
            ActorMessage::Ping { respond_to } => {
                let _ = respond_to.send(()).await;
                Ok(Lifecycle::Started)
            }
        }
    }

    /// Run the actor event loop.
    pub async fn run(mut self, mut rx: mpsc::Receiver<ActorMessage<S>>) -> Lifecycle {
        info!("Actor starting");

        while let Some(msg) = rx.recv().await {
            match self.receive(msg).await {
                Ok(Lifecycle::Stopped) => return Lifecycle::Stopped,
                Ok(_) => {}
                Err(e) => {
                    error!("Actor error: {:?}", e);
                    if matches!(self.restart_strategy, super::RestartStrategy::Never) {
                        return Lifecycle::Crashed;
                    }
                }
            }
        }

        info!("Actor shutdown complete");
        Lifecycle::Stopped
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestState {
        count: u32,
    }

    impl ActorState for TestState {}

    #[tokio::test]
    async fn test_actor_creation() {
        let state = TestState { count: 0 };
        let actor = Actor::new(state);
        assert_eq!(actor.state.count, 0);
    }

    #[tokio::test]
    async fn test_actor_init() {
        let state = TestState { count: 0 };
        let mut actor = Actor::new(state);
        let msg = ActorMessage::Init(TestState { count: 5 });
        let result = actor.receive(msg).await.unwrap();
        assert_eq!(actor.state.count, 5);
        assert_eq!(result, Lifecycle::Started);
    }

    #[tokio::test]
    async fn test_actor_stop() {
        let state = TestState { count: 10 };
        let mut actor = Actor::new(state);
        let msg = ActorMessage::Stop;
        let result = actor.receive(msg).await.unwrap();
        assert_eq!(result, Lifecycle::Stopped);
    }

    #[tokio::test]
    async fn test_actor_process() {
        let state = TestState { count: 0 };
        let mut actor = Actor::new(state);
        let msg = ActorMessage::Process("test work".to_string());
        let result = actor.receive(msg).await.unwrap();
        assert_eq!(result, Lifecycle::Started);
    }

    #[tokio::test]
    async fn test_actor_process_too_long() {
        let state = TestState { count: 0 };
        let mut actor = Actor::new(state);
        let work = "a".repeat(1025);
        let msg = ActorMessage::Process(work);
        let result = actor.receive(msg).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Lifecycle::Started);
    }

    #[tokio::test]
    async fn test_actor_process_multibyte_truncation() {
        let state = TestState { count: 0 };
        let mut actor = Actor::new(state);
        // "🦀" is a 4-byte UTF-8 character. 300 repetitions makes it 1200 bytes, which exceeds 256 chars.
        // Truncation should handle this safely without panicking on a non-character boundary.
        let work = "🦀".repeat(300);
        let msg = ActorMessage::Process(work);
        let result = actor.receive(msg).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Lifecycle::Started);
    }

    #[tokio::test]
    async fn test_actor_process_control_char() {
        let state = TestState { count: 0 };
        let mut actor = Actor::new(state);
        let msg = ActorMessage::Process("test\nwork".to_string());
        let result = actor.receive(msg).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Lifecycle::Started);
    }

    #[tokio::test]
    async fn test_actor_process_bidi_char() {
        let state = TestState { count: 0 };
        let mut actor = Actor::new(state);
        let msg = ActorMessage::Process("test\u{202a}work".to_string());
        let result = actor.receive(msg).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Lifecycle::Started);
    }

    #[test]
    fn test_sanitize_payload_passthrough() {
        assert_eq!(sanitize_log_payload("hello world"), "hello world");
    }

    #[test]
    fn test_sanitize_payload_escapes_control() {
        // \n is rendered as the two characters `\n`, so a payload cannot forge a log line.
        assert_eq!(sanitize_log_payload("a\nb"), "a\\nb");
    }

    #[test]
    fn test_sanitize_payload_escapes_bidi() {
        assert_eq!(sanitize_log_payload("a\u{202a}b"), "a\\u{202a}b");
    }

    #[test]
    fn test_sanitize_payload_truncates_long_input() {
        let out = sanitize_log_payload(&"a".repeat(1000));
        assert!(out.ends_with("... [truncated]"));
        // Payload chars are capped at MAX_LOGGED_LEN before the suffix.
        assert!(out[..out.len() - "... [truncated]".len()].len() <= MAX_LOGGED_LEN);
    }

    #[test]
    fn test_sanitize_payload_at_boundary_is_not_truncated() {
        let out = sanitize_log_payload(&"a".repeat(MAX_LOGGED_LEN));
        assert_eq!(out.chars().count(), MAX_LOGGED_LEN);
        assert!(!out.contains("truncated"));
    }

    #[test]
    fn test_sanitize_payload_never_splits_an_escape() {
        // 255 printable chars + a \n (2 escaped chars) would exceed the budget; the whole
        // `\n` escape must be dropped instead of being emitted half-way.
        let out = sanitize_log_payload(&format!("{}a\n", "a".repeat(254)));
        assert!(out.ends_with("... [truncated]"));
        assert!(
            !out.contains('\\'),
            "partial escape must never be emitted: {out}"
        );
    }

    #[test]
    fn test_sanitize_payload_multibyte_safe() {
        let out = sanitize_log_payload(&"🦀".repeat(300));
        assert!(out.ends_with("... [truncated]"));
        assert!(out.chars().count() <= MAX_LOGGED_LEN + "... [truncated]".len());
    }

    #[test]
    fn test_sanitize_payload_escaped_after_limit() {
        // First character needing escaping is after MAX_LOGGED_LEN (256)
        let input = format!("{}{}", "a".repeat(300), "\n");
        let out = sanitize_log_payload(&input);
        assert_eq!(
            out,
            format!("{}... [truncated]", "a".repeat(MAX_LOGGED_LEN))
        );
    }

    #[test]
    fn test_sanitize_payload_starts_with_escape() {
        let out = sanitize_log_payload("\nhello");
        assert_eq!(out, "\\nhello");
    }

    #[test]
    fn test_sanitize_payload_empty() {
        let out = sanitize_log_payload("");
        assert_eq!(out, "");
    }

    #[test]
    fn test_sanitize_payload_slow_path_truncation() {
        let input = format!("{}\n{}", "a".repeat(250), "b".repeat(20));
        let out = sanitize_log_payload(&input);
        assert!(out.ends_with("... [truncated]"));
        assert!(out.starts_with(&"a".repeat(250)));
    }

    #[tokio::test]
    async fn test_actor_ping() {
        let state = TestState { count: 0 };
        let mut actor = Actor::new(state);
        let (tx, mut rx) = mpsc::channel::<()>(1);
        let msg = ActorMessage::Ping { respond_to: tx };
        let result = actor.receive(msg).await.unwrap();
        assert_eq!(result, Lifecycle::Started);
        let _ = rx.recv().await;
    }

    #[tokio::test]
    async fn test_actor_runtime() {
        let state = TestState { count: 0 };
        let (tx, rx) = mpsc::channel::<ActorMessage<TestState>>(1);
        let handle = tokio::spawn(async { Actor::new(state).run(rx).await });

        tx.send(ActorMessage::Stop).await.unwrap();
        let result = handle.await.unwrap();
        assert_eq!(result, Lifecycle::Stopped);
    }
}
