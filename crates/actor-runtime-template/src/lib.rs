//! # Actor Runtime Template
//!
//! A template crate demonstrating the actor model pattern using message passing
//! for concurrent, fault-tolerant systems.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐     ┌──────────────────┐
//! │   Supervisor    │ ◀─▶ │      Actor       │
//! │ (restart policy)│     │ (mailbox, state) │
//! └─────────────────┘     └──────────────────┘
//!        │
//!        ▼
//! ┌─────────────────┐
//! │ Child Actors    │
//! └─────────────────┘
//! ```
//!
//! ## Features
//!
//! - Message types for communication
//! - State management with async handlers
//! - Supervision trees with restart policies
//! - Graceful shutdown

#![forbid(unsafe_code)]

pub mod actor;
pub mod supervisor;

use std::fmt::Debug;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::error;

/// Actor lifecycle events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lifecycle {
    /// Actor started.
    Started,
    /// Actor stopped.
    Stopped,
    /// Actor restarted.
    Restarted,
    /// Actor crashed.
    Crashed,
}

/// Actor error types.
#[derive(Debug, Error)]
pub enum ActorError {
    /// Actor panicked during execution.
    #[error("Actor panic: {0}")]
    Panic(String),

    /// Actor timed out.
    #[error("Actor timeout after {0:?}")]
    Timeout(Duration),

    /// Actor mailbox is full.
    #[error("Actor mailbox full")]
    MailboxFull,

    /// Actor state error.
    #[error("Actor state error: {0}")]
    State(String),
}

/// Supervision strategy for restart policies.
#[derive(Debug, Clone, Copy, Default)]
pub enum RestartStrategy {
    /// Always restart the actor.
    #[default]
    Always,
    /// Restart with exponential backoff.
    Backoff {
        /// Initial backoff duration.
        initial: Duration,
        /// Maximum backoff duration.
        max: Duration,
        /// Maximum restart attempts.
        max_retries: u32,
    },
    /// Never restart, just stop.
    Never,
    /// Restart only for specific error types.
    OnError {
        /// Maximum restart attempts.
        max_retries: u32,
    },
}

impl RestartStrategy {
    /// Calculate next backoff duration.
    pub fn next_backoff(&self, attempt: u32) -> Option<Duration> {
        match self {
            RestartStrategy::Backoff {
                initial,
                max,
                max_retries,
            } => {
                if attempt >= *max_retries {
                    return None;
                }
                let multiplier = 2u32.saturating_pow(attempt);
                let initial_ms = initial.as_millis();
                let max_ms = max.as_millis();
                let backoff_ms = initial_ms.saturating_mul(u128::from(multiplier));
                let capped = backoff_ms.min(max_ms);
                #[allow(clippy::cast_possible_truncation)]
                Some(Duration::from_millis(capped as u64))
            }
            RestartStrategy::Always | RestartStrategy::OnError { .. } | RestartStrategy::Never => {
                None
            }
        }
    }
}

/// State transition event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateTransition<T> {
    /// State initialized.
    Init(T),
    /// State updated.
    Update(T),
    /// State reset.
    Reset,
}

/// Actor state trait.
pub trait ActorState: Send + Sync + Clone + 'static {
    /// Validate state transitions.
    fn validate(&self) -> Result<(), ActorError> {
        let _ = self;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

    use super::*;

    #[test]
    fn test_restart_strategy_backoff() {
        let strategy = RestartStrategy::Backoff {
            initial: Duration::from_millis(100),
            max: Duration::from_millis(1000),
            max_retries: 3,
        };

        let first = strategy.next_backoff(0).unwrap();
        assert_eq!(first, Duration::from_millis(100));

        let second = strategy.next_backoff(1).unwrap();
        assert_eq!(second, Duration::from_millis(200));
    }

    #[test]
    fn test_restart_strategy_never_returns_none() {
        let strategy = RestartStrategy::Never;
        assert!(strategy.next_backoff(0).is_none());
    }

    #[test]
    fn test_backoff_at_max_retries_returns_none() {
        let strategy = RestartStrategy::Backoff {
            initial: Duration::from_millis(100),
            max: Duration::from_millis(1000),
            max_retries: 1,
        };
        assert!(strategy.next_backoff(1).is_none());
    }

    #[derive(Debug, Clone)]
    struct CustomState;
    impl ActorState for CustomState {}

    #[test]
    fn test_actor_state_validate() {
        let state = CustomState;
        assert!(state.validate().is_ok());
    }
}
