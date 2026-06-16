//! # Hybrid Storage Template
//!
//! A template crate demonstrating hybrid storage with SQL + KV backends
//! and optional caching. Based on patterns from `rust-self-learning-memory`
//! and `chaotic_semantic_memory`.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │        Backend Trait                 │
//! │  (get, set, delete, list_keys)      │
//! └─────────────────────────────────────┘
//!           │
//!     ├─────┴─────┤
//!     ▼           ▼
//! ┌───────┐   ┌───────┐
//! │ SQLite │   │   KV  │
//! │ (libSQL)│   │ (redb)│
//! └───────┘   └───────┘
//! ```
//!
//! ## Features
//!
//! - Backend trait abstraction
//! - SQL backend with libSQL/Turso support
//! - KV backend with redb
//! - Cache layer with TTL
//! - Feature-gated backend selection

pub mod backends;

use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;

/// Storage error types.
#[derive(Debug, Error)]
pub enum StorageError {
    /// Key not found.
    #[error("Key not found: {0}")]
    NotFound(String),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Connection error.
    #[error("Connection error: {0}")]
    Connection(String),

    /// Backend error.
    #[error("Backend error: {0}")]
    Backend(String),

    /// Mutex poisoned (thread panicked while holding lock).
    #[error("Internal lock poisoned")]
    Poisoned,
}

/// Backend trait for storage implementations.
#[async_trait]
pub trait Backend: Send + Sync {
    /// Get a value by key.
    async fn get(&self, key: &str) -> Result<Option<String>, StorageError>;

    /// Set a value by key.
    async fn set(&self, key: &str, value: &str) -> Result<(), StorageError>;

    /// Delete a value by key.
    async fn delete(&self, key: &str) -> Result<bool, StorageError>;

    /// List keys matching prefix.
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>, StorageError>;
}

/// Hybrid storage combining multiple backends.
pub struct HybridStorage {
    primary: Arc<dyn Backend>,
}

impl HybridStorage {
    /// Create hybrid storage with primary backend.
    pub fn new(primary: Arc<dyn Backend>) -> Self {
        Self { primary }
    }

    /// Get value.
    pub async fn get(&self, key: &str) -> Result<Option<String>, StorageError> {
        self.primary.get(key).await
    }

    /// Set value.
    pub async fn set(&self, key: &str, value: &str) -> Result<(), StorageError> {
        self.primary.set(key, value).await
    }

    /// Delete value.
    pub async fn delete(&self, key: &str) -> Result<bool, StorageError> {
        self.primary.delete(key).await
    }
}

impl Default for HybridStorage {
    fn default() -> Self {
        Self::new(Arc::new(backends::MemoryBackend::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use backends::MemoryBackend;

    #[tokio::test]
    async fn test_memory_backend_basic() {
        let backend = MemoryBackend::new();
        backend.set("key", "value").await.unwrap();

        let result = backend.get("key").await.unwrap();
        assert_eq!(result, Some("value".to_string()));
    }

    #[tokio::test]
    async fn test_hybrid_storage_basic() {
        let storage = HybridStorage::default();
        storage.set("foo", "bar").await.unwrap();

        let result = storage.get("foo").await.unwrap();
        assert_eq!(result, Some("bar".to_string()));
    }

    #[tokio::test]
    async fn test_hybrid_storage_delete() {
        let storage = HybridStorage::default();
        storage.set("delete", "me").await.unwrap();

        let deleted = storage.delete("delete").await.unwrap();
        assert!(deleted);

        let result = storage.get("delete").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_hybrid_storage_not_found() {
        let storage = HybridStorage::default();
        let result = storage.get("missing").await.unwrap();
        assert!(result.is_none());
    }
}
