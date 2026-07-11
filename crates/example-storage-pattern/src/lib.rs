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

#![forbid(unsafe_code)]

use std::future::Future;
use std::pin::Pin;

/// Minimal CRUD surface. Implement this trait for each backend.
/// Consumer code depends only on `dyn Backend`, never on a concrete type.
pub trait Backend: Send + Sync {
    /// The error type returned by backend operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Get a value by key.
    fn get(
        &self,
        key: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<String>, Self::Error>> + Send + '_>>;
    /// Set a value by key.
    fn set(
        &self,
        key: &str,
        value: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + '_>>;
    /// Delete a value by key.
    fn delete(
        &self,
        key: &str,
    ) -> Pin<Box<dyn Future<Output = Result<bool, Self::Error>> + Send + '_>>;
}

/// In-memory mock backend for use in tests. Never use in production.
#[cfg(any(test, feature = "mock"))]
pub mod mock {
    use super::{Backend, Future, Pin};
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// In-memory backend for testing.
    #[derive(Default)]
    pub struct MockBackend(Mutex<HashMap<String, String>>);

    /// Error type for mock backend operations.
    #[derive(Debug, Clone, thiserror::Error)]
    #[error("mock error: {0}")]
    pub struct MockError(String);

    impl Backend for MockBackend {
        type Error = MockError;
        fn get(
            &self,
            key: &str,
        ) -> Pin<Box<dyn Future<Output = Result<Option<String>, MockError>> + Send + '_>> {
            let key = key.to_string();
            Box::pin(async move {
                Ok(self
                    .0
                    .lock()
                    .map_err(|_| MockError("mutex poisoned".into()))?
                    .get(&key)
                    .cloned())
            })
        }
        fn set(
            &self,
            key: &str,
            value: &str,
        ) -> Pin<Box<dyn Future<Output = Result<(), MockError>> + Send + '_>> {
            let key = key.to_string();
            let value = value.to_string();
            Box::pin(async move {
                self.0
                    .lock()
                    .map_err(|_| MockError("mutex poisoned".into()))?
                    .insert(key, value);
                Ok(())
            })
        }
        fn delete(
            &self,
            key: &str,
        ) -> Pin<Box<dyn Future<Output = Result<bool, MockError>> + Send + '_>> {
            let key = key.to_string();
            Box::pin(async move {
                Ok(self
                    .0
                    .lock()
                    .map_err(|_| MockError("mutex poisoned".into()))?
                    .remove(&key)
                    .is_some())
            })
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

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
