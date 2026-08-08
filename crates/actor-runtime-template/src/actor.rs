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
                // Security: Prevent log injection and log-filling DoS by escaping control/Bidi characters
                // and limiting the logged string length at the logging boundary.
                const MAX_LOGGED_LEN: usize = 256;
                let escaped = work.escape_debug().to_string();
                if escaped.chars().count() > MAX_LOGGED_LEN {
                    let mut truncated: String = escaped.chars().take(MAX_LOGGED_LEN).collect();
                    truncated.push_str("... [truncated]");
                    debug!("Processing: {}", truncated);
                } else {
                    debug!("Processing: {}", escaped);
                }

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
