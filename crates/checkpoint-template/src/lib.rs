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

#![forbid(unsafe_code)]

pub mod migration;
pub mod storage;

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::path::Path;
use std::time::SystemTime;
use thiserror::Error;
use tracing::info;

pub use migration::MigrationError;
use storage::{DEFAULT_MAX_CHECKPOINT_SIZE, FileStorage};

/// Checkpoint header stored with each checkpoint file.
/// Checkpoint header stored with each checkpoint file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CheckpointHeader {
    /// Schema version of the checkpoint.
    pub version: u32,
    /// Timestamp when the checkpoint was created.
    pub created_at: SystemTime,
    /// Name of the application that created the checkpoint.
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

/// Security configuration for checkpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointConfig {
    /// Maximum size of a checkpoint file in bytes.
    pub max_checkpoint_size: u64,
    /// Maximum length of the app_name string in the header.
    pub max_app_name_len: usize,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            max_checkpoint_size: DEFAULT_MAX_CHECKPOINT_SIZE,
            // 256 bytes default limit
            max_app_name_len: 256,
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
    config: CheckpointConfig,
    storage: FileStorage,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Storable> CheckpointManager<T> {
    /// Create a new checkpoint manager with default configuration.
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            header: CheckpointHeader::default(),
            config: CheckpointConfig::default(),
            storage: FileStorage::new(path),
            _marker: std::marker::PhantomData,
        }
    }

    /// Create a new checkpoint manager with custom configuration.
    pub fn with_config(path: impl AsRef<Path>, config: CheckpointConfig) -> Self {
        Self {
            header: CheckpointHeader::default(),
            config,
            storage: FileStorage::new(path),
            _marker: std::marker::PhantomData,
        }
    }

    /// Save state atomically.
    pub async fn save(&mut self, state: &T) -> Result<(), CheckpointError> {
        self.header.version = T::version();
        self.header.created_at = SystemTime::now();

        use bincode::Options;
        let options = bincode::options();

        let data = options
            .serialize(state)
            .map_err(|e| CheckpointError::Serialization(e.to_string()))?;

        self.storage.save(&self.header, &data).await?;
        info!("Checkpoint saved (version {})", self.header.version);
        Ok(())
    }

    /// Load state with migration support.
    pub async fn load(&self) -> Result<Option<T>, CheckpointError> {
        let (header, payload) = match self
            .storage
            .load_with_limit(self.config.max_checkpoint_size)
            .await
        {
            Ok(v) => v,
            Err(storage::StorageError::NotFound) => return Ok(None),
            Err(e) => return Err(CheckpointError::Storage(e)),
        };

        // Security (2026): Sanitize app_name to prevent log injection and resource exhaustion.
        if header.app_name.len() > self.config.max_app_name_len {
            return Err(CheckpointError::Serialization(format!(
                "app_name too long: {} bytes (max {})",
                header.app_name.len(),
                self.config.max_app_name_len
            )));
        }

        if header.app_name.chars().any(|c| {
            c.is_control()
                || matches!(
                    c,
                    '\u{200b}'..='\u{200f}'
                        | '\u{202a}'..='\u{202e}'
                        | '\u{2066}'..='\u{2069}'
                )
        }) {
            return Err(CheckpointError::Serialization(
                "app_name contains control or Bidi characters".to_string(),
            ));
        }

        // Handle version mismatch
        if header.version != T::version() {
            return Err(CheckpointError::VersionMismatch {
                expected: T::version(),
                actual: header.version,
            });
        }

        // Security: Use bincode with a size limit to prevent resource exhaustion.
        use bincode::Options;
        let options = bincode::options().with_limit(self.config.max_checkpoint_size);

        let state = options
            .deserialize(&payload)
            .map_err(|e| CheckpointError::Serialization(e.to_string()))?;

        Ok(Some(state))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

    use super::*;
    use tempfile::TempDir;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestState {
        value: u32,
    }

    impl Storable for TestState {
        fn version() -> u32 {
            1
        }
    }

    #[tokio::test]
    async fn test_load_version_mismatch() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("version_mismatch.ckpt");

        let header = CheckpointHeader {
            version: 99, // Mismatched version
            created_at: SystemTime::UNIX_EPOCH,
            app_name: "test".to_string(),
        };
        let state = TestState { value: 42 };

        use bincode::Options;
        let options = bincode::options();
        let state_data = options.serialize(&state).unwrap();
        let mut combined = options.serialize(&header).unwrap();
        combined.extend_from_slice(&state_data);
        std::fs::write(&path, combined).unwrap();

        let manager = CheckpointManager::<TestState>::new(&path);
        let result = manager.load().await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, CheckpointError::VersionMismatch { .. }));
    }

    #[tokio::test]
    async fn test_checkpoint_manager_save_load() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.ckpt");

        let mut manager = CheckpointManager::new(&path);
        let state = TestState { value: 42 };

        manager.save(&state).await.unwrap();
        let loaded = manager.load().await.unwrap();
        assert_eq!(loaded, Some(state));
    }

    #[tokio::test]
    async fn test_checkpoint_manager_not_found() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.ckpt");

        let manager = CheckpointManager::<TestState>::new(&path);
        let result = manager.load().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_load_config_too_large() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("too_large.ckpt");

        // Create a file larger than default 10MB
        let large_data = vec![0u8; 11 * 1024 * 1024];
        std::fs::write(&path, large_data).unwrap();

        let manager = CheckpointManager::<TestState>::new(&path);
        let result = manager.load().await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            CheckpointError::Storage(storage::StorageError::TooLarge(_))
        ));
    }

    #[tokio::test]
    async fn test_load_config_invalid_type() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("directory.ckpt");
        std::fs::create_dir(&path).unwrap();

        let manager = CheckpointManager::<TestState>::new(&path);
        let result = manager.load().await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            CheckpointError::Storage(storage::StorageError::InvalidType)
        ));
    }

    #[tokio::test]
    async fn test_load_config_app_name_too_long() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("long_app_name.ckpt");

        let header = CheckpointHeader {
            version: 1,
            created_at: SystemTime::UNIX_EPOCH,
            app_name: "a".repeat(257),
        };
        let state = TestState { value: 42 };

        use bincode::Options;
        let options = bincode::options();
        let state_data = options.serialize(&state).unwrap();

        let mut combined = options.serialize(&header).unwrap();
        combined.extend_from_slice(&state_data);

        std::fs::write(&path, combined).unwrap();

        let manager = CheckpointManager::<TestState>::new(&path);
        let result = manager.load().await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("app_name too long"));
    }

    #[tokio::test]
    async fn test_load_config_app_name_control_chars() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("control_chars.ckpt");

        let header = CheckpointHeader {
            version: 1,
            created_at: SystemTime::UNIX_EPOCH,
            app_name: "test\napp".to_string(),
        };
        let state = TestState { value: 42 };

        use bincode::Options;
        let options = bincode::options();
        let state_data = options.serialize(&state).unwrap();

        let mut combined = options.serialize(&header).unwrap();
        combined.extend_from_slice(&state_data);

        std::fs::write(&path, combined).unwrap();

        let manager = CheckpointManager::<TestState>::new(&path);
        let result = manager.load().await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("control or Bidi characters"));
    }

    #[tokio::test]
    async fn test_load_config_app_name_bidi_chars() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bidi_chars.ckpt");

        let header = CheckpointHeader {
            version: 1,
            created_at: SystemTime::UNIX_EPOCH,
            // Use a Bidi control character (U+202A: LEFT-TO-RIGHT EMBEDDING)
            app_name: "test\u{202a}app".to_string(),
        };
        let state = TestState { value: 42 };

        use bincode::Options;
        let options = bincode::options();
        let state_data = options.serialize(&state).unwrap();

        let mut combined = options.serialize(&header).unwrap();
        combined.extend_from_slice(&state_data);

        std::fs::write(&path, combined).unwrap();

        let manager = CheckpointManager::<TestState>::new(&path);
        let result = manager.load().await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("control or Bidi characters"));
    }

    #[tokio::test]
    async fn test_with_custom_config() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("custom.ckpt");

        let config = CheckpointConfig {
            max_checkpoint_size: 100,
            max_app_name_len: 5,
        };

        let manager = CheckpointManager::<TestState>::with_config(&path, config);

        // App name too long for custom config
        let header = CheckpointHeader {
            version: 1,
            created_at: SystemTime::UNIX_EPOCH,
            app_name: "too_long".to_string(),
        };
        let state = TestState { value: 42 };

        use bincode::Options;
        let options = bincode::options();
        let state_data = options.serialize(&state).unwrap();
        let mut combined = options.serialize(&header).unwrap();
        combined.extend_from_slice(&state_data);
        std::fs::write(&path, combined).unwrap();

        let result = manager.load().await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("app_name too long")
        );
    }
}
