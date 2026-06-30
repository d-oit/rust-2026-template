//! Checkpoint storage backends.

use super::CheckpointHeader;
pub use super::MigrationError;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// Default maximum size for a checkpoint file (10MB).
pub const DEFAULT_MAX_CHECKPOINT_SIZE: u64 = 10 * 1024 * 1024;

/// Storage error types.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// IO error.
    #[error("IO error: {0}")]
    Io(#[source] std::io::Error),

    /// Serialization error.
    #[error("Serialization error")]
    Serialization,

    /// Checkpoint not found.
    #[error("Checkpoint not found")]
    NotFound,

    /// Checkpoint file too large.
    #[error("Checkpoint too large: {0} bytes")]
    TooLarge(u64),

    /// Invalid file type.
    #[error("Invalid file type")]
    InvalidType,
}

/// File-based checkpoint storage with atomic writes.
pub struct FileStorage {
    path: PathBuf,
}

impl FileStorage {
    /// Create a new file storage.
    pub fn new(path: impl AsRef<std::path::Path>) -> Self {
        Self {
            path: path.as_ref().to_owned(),
        }
    }

    /// Save checkpoint data.
    pub async fn save(&self, header: &CheckpointHeader, data: &[u8]) -> Result<(), StorageError> {
        let temp_path = self.path.with_extension("tmp");
        let final_path = self.path.with_extension("ckpt");

        use bincode::Options;
        let options = bincode::options();

        // Security (2026): Write header and data sequentially to avoid large intermediate Vec
        let header_bytes = options
            .serialize(header)
            .map_err(|_| StorageError::Serialization)?;

        let mut file = fs::File::create(&temp_path)
            .await
            .map_err(StorageError::Io)?;

        file.write_all(&header_bytes)
            .await
            .map_err(StorageError::Io)?;
        file.write_all(data).await.map_err(StorageError::Io)?;
        file.sync_all().await.map_err(StorageError::Io)?;

        fs::rename(&temp_path, &final_path)
            .await
            .map_err(StorageError::Io)?;

        Ok(())
    }

    /// Load checkpoint data with a specific size limit.
    pub async fn load_with_limit(
        &self,
        max_size: u64,
    ) -> Result<(CheckpointHeader, Vec<u8>), StorageError> {
        let final_path = self.path.with_extension("ckpt");

        // Security (2026): Open file FIRST to avoid TOCTOU (Time-of-Check to Time-of-Use)
        // vulnerabilities. Using the file handle for subsequent metadata checks.
        let file = fs::File::open(&final_path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound
            } else {
                StorageError::Io(e)
            }
        })?;

        let metadata = file.metadata().await.map_err(StorageError::Io)?;

        if !metadata.is_file() {
            return Err(StorageError::InvalidType);
        }

        let file_size = metadata.len();
        if file_size > max_size {
            return Err(StorageError::TooLarge(file_size));
        }

        // Security (2026): Use a capacity-limited reader to prevent OOM
        // if the file grows between metadata check and read (though rare on handles).
        let data_capacity =
            usize::try_from(file_size).map_err(|_| StorageError::TooLarge(file_size))?;
        let mut data = Vec::with_capacity(data_capacity);

        use tokio::io::AsyncReadExt;
        let mut reader = file.take(max_size);
        reader
            .read_to_end(&mut data)
            .await
            .map_err(StorageError::Io)?;

        let mut cursor = std::io::Cursor::new(&data);
        // Security: Use bincode with a size limit to prevent resource exhaustion.
        use bincode::Options;
        let options = bincode::options()
            .with_limit(max_size)
            .allow_trailing_bytes();

        let header: CheckpointHeader = options
            .deserialize_from(&mut cursor)
            .map_err(|_| StorageError::Serialization)?;

        let payload_start =
            usize::try_from(cursor.position()).map_err(|_| StorageError::Serialization)?;
        data.drain(..payload_start);
        Ok((header, data))
    }

    /// Load checkpoint data using default limit (10MB).
    pub async fn load(&self) -> Result<(CheckpointHeader, Vec<u8>), StorageError> {
        self.load_with_limit(DEFAULT_MAX_CHECKPOINT_SIZE).await
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_storage() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.ckpt");

        let storage = FileStorage::new(&path);
        let header = CheckpointHeader::default();
        let data = b"test data".to_vec();

        storage.save(&header, &data).await.unwrap();

        let (loaded_header, _loaded_data) = storage.load().await.unwrap();
        assert_eq!(loaded_header.version, 1);
    }

    #[tokio::test]
    async fn test_file_storage_not_found() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.ckpt");

        let storage = FileStorage::new(&path);
        let result = storage.load().await;
        assert!(matches!(result, Err(StorageError::NotFound)));
    }
}
