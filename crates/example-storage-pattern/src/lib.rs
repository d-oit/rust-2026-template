//! # Trait-Only Storage Pattern
//!
//! Demonstrates the **trait-only storage layer**: the `Backend` trait lives in
//! this crate with zero implementations. Concrete backends (`SqliteBackend`,
//! future `PostgresBackend`) implement the trait behind a feature flag without
//! touching any consumer crate.
//!
//! ## When to use
//! - You want to swap storage backends without changing business logic
//! - You need fast, zero-I/O unit tests via a `MockBackend`
//! - You publish a library and don't want to force a storage dependency on users
//!
//! ## Structure
//! ```text
//! your-types  ←  your-storage-trait  ←  your-sqlite-backend
//!                                    ←  your-mock-backend (cfg(test))
//!                     ↑
//!              your-business-logic (depends only on the trait)
//! ```

use async_trait::async_trait;

/// Minimal CRUD surface. Implement this trait for each backend.
/// Consumer code depends only on `dyn Backend`, never on a concrete type.
#[async_trait]
pub trait Backend: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn get(&self, key: &str) -> Result<Option<String>, Self::Error>;
    async fn set(&self, key: &str, value: &str) -> Result<(), Self::Error>;
    async fn delete(&self, key: &str) -> Result<bool, Self::Error>;
}

/// In-memory mock backend for use in tests. Never use in production.
#[cfg(any(test, feature = "mock"))]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    pub struct MockBackend(Mutex<HashMap<String, String>>);

    #[derive(Debug, thiserror::Error)]
    #[error("mock error: {0}")]
    pub struct MockError(String);

    #[async_trait]
    impl Backend for MockBackend {
        type Error = MockError;
        async fn get(&self, key: &str) -> Result<Option<String>, MockError> {
            Ok(self.0.lock().unwrap().get(key).cloned())
        }
        async fn set(&self, key: &str, value: &str) -> Result<(), MockError> {
            self.0.lock().unwrap().insert(key.into(), value.into());
            Ok(())
        }
        async fn delete(&self, key: &str) -> Result<bool, MockError> {
            Ok(self.0.lock().unwrap().remove(key).is_some())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Backend, mock::MockBackend};

    #[tokio::test]
    async fn roundtrip() {
        let b = MockBackend::default();
        b.set("k", "v").await.unwrap();
        assert_eq!(b.get("k").await.unwrap(), Some("v".into()));
        assert!(b.delete("k").await.unwrap());
        assert_eq!(b.get("k").await.unwrap(), None);
    }
}
