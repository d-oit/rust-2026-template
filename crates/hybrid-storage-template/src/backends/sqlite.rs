//! SQLite backend implementation using libSQL.

use crate::StorageError;
use async_trait::async_trait;

/// SQLite backend using libSQL/Turso.
pub struct SqliteBackend {
    _inner: Option<libsql::Database>,
}

impl SqliteBackend {
    /// Create a new SQLite backend.
    pub async fn new(_url: &str) -> Result<Self, StorageError> {
        // Placeholder - actual implementation would connect to libSQL
        Ok(Self { _inner: None })
    }
}

#[async_trait]
impl crate::Backend for SqliteBackend {
    async fn get(&self, _key: &str) -> Result<Option<String>, StorageError> {
        Ok(None)
    }

    async fn set(&self, _key: &str, _value: &str) -> Result<(), StorageError> {
        Ok(())
    }

    async fn delete(&self, _key: &str) -> Result<bool, StorageError> {
        Ok(false)
    }

    async fn list_keys(&self, _prefix: &str) -> Result<Vec<String>, StorageError> {
        Ok(Vec::new())
    }
}
