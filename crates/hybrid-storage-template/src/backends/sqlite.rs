//! SQLite backend implementation using libSQL.
//!
//! This is a **stub implementation** for template purposes.
//! A real implementation would use libsql to connect to a SQLite database.
//! See: https://docs.rs/libsql

use crate::StorageError;
use std::future::Future;
use std::pin::Pin;

/// SQLite backend using libSQL/Turso.
///
/// This is a placeholder implementation. For a real backend, you would:
/// 1. Store a `libsql::Database` connection
/// 2. Implement proper CRUD operations using SQL queries
/// 3. Handle connection pooling and error recovery
pub struct SqliteBackend {
    _inner: Option<libsql::Database>,
}

impl SqliteBackend {
    /// Create a new SQLite backend.
    ///
    /// The `url` parameter should be a libSQL connection URL, e.g.:
    /// - `"file::memory:?cache=shared"` for in-memory
    /// - `"libsql://your-db.turso.io"` for remote
    pub async fn new(_url: &str) -> Result<Self, StorageError> {
        // TODO: Implement actual libSQL connection
        // let db = libsql::Database::new(url).await?;
        // Ok(Self { inner: Some(db) })
        Ok(Self { _inner: None })
    }
}

impl crate::Backend for SqliteBackend {
    fn get(
        &self,
        _key: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<String>, StorageError>> + Send + '_>> {
        Box::pin(async { Ok(None) })
    }

    fn set(
        &self,
        _key: &str,
        _value: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), StorageError>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }

    fn delete(
        &self,
        _key: &str,
    ) -> Pin<Box<dyn Future<Output = Result<bool, StorageError>> + Send + '_>> {
        Box::pin(async { Ok(false) })
    }

    fn list_keys(
        &self,
        _prefix: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, StorageError>> + Send + '_>> {
        Box::pin(async { Ok(Vec::new()) })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

    use super::*;
    use crate::Backend;

    #[tokio::test]
    async fn test_sqlite_backend_placeholder() {
        let backend = SqliteBackend::new("file::memory:?cache=shared")
            .await
            .unwrap();
        assert!(backend.get("test").await.unwrap().is_none());
        assert!(backend.set("test", "value").await.is_ok());
        assert!(!backend.delete("test").await.unwrap());
        assert!(backend.list_keys("").await.unwrap().is_empty());
    }
}
