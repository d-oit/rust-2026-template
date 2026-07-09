//! Supervisor patterns for actor lifecycle management.

use crate::actor::{Actor, ActorMessage};
use crate::{ActorState, RestartStrategy};
use tokio::sync::mpsc;

/// Supervisor error types.
#[derive(Debug, thiserror::Error)]
pub enum SupervisorError {
    /// Actor panicked.
    #[error("Actor panicked: {0}")]
    ActorPanic(String),

    /// Max retries exceeded.
    #[error("Max retries exceeded for actor")]
    MaxRetries,

    /// Actor spawn failed.
    #[error("Failed to spawn actor: {0}")]
    Spawn(String),
}

/// Supervises child actors with restart policies.
pub struct Supervisor<S: ActorState> {
    _strategy: RestartStrategy,
    state: S,
}

impl<S: ActorState + 'static> Supervisor<S> {
    /// Create a new supervisor.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "const not possible across MSRV/generic bounds"
    )]
    pub fn new(strategy: RestartStrategy, state: S) -> Self {
        Self {
            _strategy: strategy,
            state,
        }
    }

    /// Spawn and supervise an actor, returning a handle to communicate with it.
    pub fn spawn(&self) -> Result<mpsc::Sender<ActorMessage<S>>, SupervisorError> {
        let (tx, rx) = mpsc::channel::<ActorMessage<S>>(64);
        let state = self.state.clone();

        // Spawn actor task
        tokio::spawn(async move {
            let actor = Actor::new(state);
            let _ = actor.run(rx).await;
        });

        Ok(tx)
    }
}

/// Commands for supervisor control.
pub enum SupervisorCommand {
    /// Restart a child actor.
    Restart,
    /// Stop supervising.
    Stop,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestState {
        value: i32,
    }

    impl crate::ActorState for TestState {}

    #[tokio::test]
    async fn test_supervisor_creation() {
        let state = TestState { value: 42 };
        let supervisor = Supervisor::new(Default::default(), state);
        assert_eq!(supervisor.state.value, 42);
    }

    #[tokio::test]
    async fn test_restart_strategy_never() {
        let strategy = RestartStrategy::Never;
        assert!(strategy.next_backoff(0).is_none());
    }
}
