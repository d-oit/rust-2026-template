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
