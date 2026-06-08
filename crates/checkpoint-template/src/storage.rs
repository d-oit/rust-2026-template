//! Checkpoint storage backends.

use super::CheckpointHeader;
pub use super::MigrationError;
use std::path::PathBuf;
use std::time::SystemTime;
use tokio::fs;
use tokio::io::AsyncWriteExt;

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

        let combined: Vec<u8> =
            bincode::serialize(&(header.version, header.created_at, header.app_name.clone()))
                .map_err(|_| StorageError::Serialization)?
                .into_iter()
                .chain(data.iter().copied())
                .collect();

        let mut file = fs::File::create(&temp_path)
            .await
            .map_err(StorageError::Io)?;
        file.write_all(&combined).await.map_err(StorageError::Io)?;
        file.sync_all().await.map_err(StorageError::Io)?;

        fs::rename(&temp_path, &final_path)
            .await
            .map_err(StorageError::Io)?;

        Ok(())
    }

    /// Load checkpoint data.
    pub async fn load(&self) -> Result<(CheckpointHeader, Vec<u8>), StorageError> {
        let final_path = self.path.with_extension("ckpt");
        let data = fs::read(&final_path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound
            } else {
                StorageError::Io(e)
            }
        })?;

        let mut cursor = std::io::Cursor::new(&data);
        let (version, created_at, app_name): (u32, SystemTime, String) =
            bincode::deserialize_from(&mut cursor).map_err(|_| StorageError::Serialization)?;

        let header = CheckpointHeader {
            version,
            created_at,
            app_name,
        };

        let payload = data[usize::try_from(cursor.position()).unwrap_or(0)..].to_vec();
        Ok((header, payload))
    }
}

#[cfg(test)]
mod tests {
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
