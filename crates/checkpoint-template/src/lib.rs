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
use std::path::Path;
use std::time::SystemTime;
use thiserror::Error;
use tracing::info;

pub use migration::MigrationError;
use storage::{DEFAULT_MAX_CHECKPOINT_SIZE, FileStorage};

/// Checkpoint header stored with each checkpoint file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    /// Maximum length of the `app_name` string in the header.
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

    /// Migrate a payload written by an older schema version.
    ///
    /// `payload` is the raw stored bytes (bincode), **not** JSON. JSON is not
    /// available here because bincode is not self-describing and cannot decode
    /// an arbitrary `serde_json::Value`; a previous signature that took
    /// `serde_json::Value` could therefore never be implemented and was never
    /// called.
    ///
    /// The default implementation returns the payload unchanged, which means
    /// "no migration declared": [`CheckpointManager::load`] then reports
    /// [`CheckpointError::VersionMismatch`] rather than guessing a layout.
    /// Override this to decode `from_version`'s layout, transform it, and
    /// re-encode the current layout.
    ///
    /// # Errors
    ///
    /// Returns [`MigrationError`] when the payload cannot be migrated.
    fn migrate(payload: &[u8], from_version: u32) -> Result<Vec<u8>, MigrationError>
    where
        Self: Sized,
    {
        let _ = from_version;
        Ok(payload.to_vec())
    }

    /// Whether [`Self::migrate`] can handle `from_version`.
    ///
    /// Defaults to `false` so a type only opts into migration explicitly.
    #[must_use]
    fn supports_migration_from(from_version: u32) -> bool
    where
        Self: Sized,
    {
        let _ = from_version;
        false
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

    /// Set the application name with security validation.
    ///
    /// # Errors
    ///
    /// Returns [`CheckpointError::Serialization`] when the name exceeds
    /// [`CheckpointConfig::max_app_name_len`] or contains control/Bidi characters.
    pub fn set_app_name(&mut self, app_name: impl Into<String>) -> Result<(), CheckpointError> {
        let app_name = app_name.into();
        Self::validate_app_name(&app_name, &self.config)?;
        self.header.app_name = app_name;
        Ok(())
    }

    /// Save state atomically.
    ///
    /// # Errors
    ///
    /// Returns [`CheckpointError::Serialization`] if the state cannot be
    /// encoded or is oversized, and [`CheckpointError::Storage`] on I/O failure.
    pub async fn save(&mut self, state: &T) -> Result<(), CheckpointError> {
        // Security: Validate header before saving.
        Self::validate_app_name(&self.header.app_name, &self.config)?;

        self.header.version = T::version();
        self.header.created_at = SystemTime::now();

        // Security: Serialize then check size to prevent resource exhaustion.
        let config = bincode_reloaded::config::standard();
        let data = bincode_reloaded::serde::encode_to_vec(state, config)
            .map_err(|e| CheckpointError::Serialization(e.to_string()))?;
        if u64::try_from(data.len()).unwrap_or(u64::MAX) > self.config.max_checkpoint_size {
            return Err(CheckpointError::Serialization(format!(
                "checkpoint too large: {} bytes (max {})",
                data.len(),
                self.config.max_checkpoint_size
            )));
        }

        self.storage.save(&self.header, &data).await?;
        info!("Checkpoint saved (version {})", self.header.version);
        Ok(())
    }

    /// Validate application name for security constraints.
    fn validate_app_name(name: &str, config: &CheckpointConfig) -> Result<(), CheckpointError> {
        // Security (2026): Sanitize app_name to prevent log injection and resource exhaustion.
        if name.len() > config.max_app_name_len {
            return Err(CheckpointError::Serialization(format!(
                "app_name too long: {} bytes (max {})",
                name.len(),
                config.max_app_name_len
            )));
        }

        // Security: Robust hierarchical validation with manual byte-scan fast-path.
        // Skips UTF-8 decoding for the common case where the string is entirely printable ASCII (0x20-0x7E).
        let bytes = name.as_bytes();
        let mut i = 0;

        // SWAR (SIMD Within A Register) fast-path: check 8-byte chunks simultaneously for printable ASCII values (0x20-0x7E)
        while i + 8 <= bytes.len() {
            let mut arr = [0u8; 8];
            arr.copy_from_slice(&bytes[i..i + 8]);
            let chunk = u64::from_ne_bytes(arr);
            let has_low =
                (chunk.wrapping_sub(0x2020_2020_2020_2020) & !chunk) & 0x8080_8080_8080_8080;
            let has_high = chunk & 0x8080_8080_8080_8080;
            let y = chunk ^ 0x7F7F_7F7F_7F7F_7F7F;
            let zero_check = (y.wrapping_sub(0x0101_0101_0101_0101) & !y) & 0x8080_8080_8080_8080;
            if (has_low | has_high | zero_check) != 0 {
                break;
            }
            i += 8;
        }

        while i < bytes.len() {
            let b = bytes[i];
            if !(0x20..=0x7E).contains(&b) {
                break;
            }
            i += 1;
        }

        if i == bytes.len() {
            return Ok(());
        }

        // Slow path: Full Unicode validation for control and Bidi characters starting from first non-ASCII byte.
        for c in name[i..].chars() {
            if c.is_control()
                || matches!(
                    c,
                    '\u{200b}'..='\u{200f}' // Zero-width space and Bidi controls
                        | '\u{2028}' // Line separator
                        | '\u{2029}' // Paragraph separator
                        | '\u{202a}'..='\u{202e}' // Bidi embedding/override
                        | '\u{2060}'..='\u{2064}' // Word joiner and invisible formatters
                        | '\u{2066}'..='\u{2069}' // Bidi isolate controls
                )
            {
                return Err(CheckpointError::Serialization(
                    "app_name contains control or Bidi characters".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Load state with migration support.
    ///
    /// # Errors
    ///
    /// Returns [`CheckpointError::Storage`] on I/O or decode failure, and
    /// [`CheckpointError::VersionMismatch`] when the stored schema version
    /// differs from `T::version()`.
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

        Self::validate_app_name(&header.app_name, &self.config)?;

        // Migrate older schema versions before decoding. A type opts in through
        // `Storable::supports_migration_from`; without that, fail closed rather
        // than decode bytes written by a layout we do not understand.
        let payload = if header.version == T::version() {
            payload
        } else {
            if !T::supports_migration_from(header.version) {
                return Err(CheckpointError::VersionMismatch {
                    expected: T::version(),
                    actual: header.version,
                });
            }
            T::migrate(&payload, header.version)?
        };

        // Security: Check payload size then deserialize.
        if u64::try_from(payload.len()).unwrap_or(u64::MAX) > self.config.max_checkpoint_size {
            return Err(CheckpointError::Serialization(format!(
                "payload too large: {} bytes (max {})",
                payload.len(),
                self.config.max_checkpoint_size
            )));
        }
        let config = bincode_reloaded::config::standard();
        let (state, _) = bincode_reloaded::serde::decode_from_slice(&payload, config)
            .map_err(|e| CheckpointError::Serialization(e.to_string()))?;

        Ok(Some(state))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_migration;
