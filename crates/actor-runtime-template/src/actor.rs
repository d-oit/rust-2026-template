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
    GetState { respond_to: mpsc::Sender<S> },
    /// Stop the actor.
    Stop,
    /// Health check ping.
    Ping { respond_to: mpsc::Sender<()> },
}

/// Actor handle for external control.
pub struct ActorHandle<S: ActorState> {
    tx: mpsc::Sender<ActorMessage<S>>,
    state: S,
}

impl<S: ActorState> ActorHandle<S> {
    /// Create a new actor handle.
    #[allow(clippy::missing_const_for_fn)]
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
    #[allow(clippy::missing_const_for_fn)]
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
                debug!("Processing: {}", work);
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
}
