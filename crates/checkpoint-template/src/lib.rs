//! # Checkpoint Template
//!
//! A template crate for serializable application state with save/restore
//! and migration support. Based on patterns from `axocoatl` and
//! `chaotic_semantic_memory`.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │      Application State              │
//! │  (implements Storable trait)        │
//! └─────────────────────────────────────┘
//!           │
//!           ▼
//! ┌─────────────────────────────────────┐
//! │      Checkpoint Manager              │
//! │  (save, load, migrate)              │
//! └─────────────────────────────────────┘
//!           │
//!           ▼
//! ┌─────────────────────────────────────┐
//! │    Storage (file, atomic ops)       │
//! └─────────────────────────────────────┘
//! ```

pub mod migration;
pub mod storage;

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::path::Path;
use std::time::SystemTime;
use thiserror::Error;
use tracing::{info, warn};

pub use migration::MigrationError;
pub use storage::FileStorage;

/// Checkpoint metadata for versioning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointHeader {
    /// Schema version.
    pub version: u32,
    /// When the checkpoint was created.
    pub created_at: SystemTime,
    /// Application name for namespacing.
    pub app_name: String,
}

impl Default for CheckpointHeader {
    fn default() -> Self {
        Self {
            version: 1,
            created_at: SystemTime::UNIX_EPOCH,
            app_name: "unknown".to_string(),
        }
    }
}

/// Checkpoint error types.
#[derive(Debug, Error)]
pub enum CheckpointError {
    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[source] std::io::Error),

    /// Migration error.
    #[error("Migration error: {0}")]
    Migration(#[from] MigrationError),

    /// Version mismatch.
    #[error("Version mismatch: expected {expected}, got {actual}")]
    VersionMismatch {
        /// Expected version.
        expected: u32,
        /// Actual version.
        actual: u32,
    },

    /// Storage error.
    #[error("Storage error: {0}")]
    Storage(#[from] storage::StorageError),
}

/// Trait for checkpoint-serializable state.
pub trait Storable: Serialize + DeserializeOwned + Send + Sync + Clone + 'static {
    /// Returns the current schema version.
    fn version() -> u32
    where
        Self: Sized;

    /// Migrate data from an older version.
    fn migrate(data: Value, from_version: u32) -> Result<Value, MigrationError>
    where
        Self: Sized,
    {
        let _ = from_version;
        Ok(data)
    }
}

/// Manages checkpoints with atomic save/load.
pub struct CheckpointManager<T: Storable> {
    header: CheckpointHeader,
    storage: FileStorage,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Storable> CheckpointManager<T> {
    /// Create a new checkpoint manager.
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            header: CheckpointHeader::default(),
            storage: FileStorage::new(path),
            _marker: std::marker::PhantomData,
        }
    }

    /// Save state atomically.
    pub async fn save(&mut self, state: &T) -> Result<(), CheckpointError> {
        self.header.version = T::version();
        self.header.created_at = SystemTime::now();

        let data =
            bincode::serialize(state).map_err(|e| CheckpointError::Serialization(e.to_string()))?;

        self.storage.save(&self.header, &data).await?;
        info!("Checkpoint saved (version {})", self.header.version);
        Ok(())
    }

    /// Load state with migration support.
    pub async fn load(&self) -> Result<Option<T>, CheckpointError> {
        let (header, data) = match self.storage.load().await {
            Ok(v) => v,
            Err(e) => {
                warn!("No checkpoint found: {}", e);
                return Ok(None);
            }
        };

        // Handle version mismatch
        if header.version != T::version() {
            return Err(CheckpointError::VersionMismatch {
                expected: T::version(),
                actual: header.version,
            });
        }

        let state = bincode::deserialize(&data)
            .map_err(|e| CheckpointError::Serialization(e.to_string()))?;

        Ok(Some(state))
    }
}
