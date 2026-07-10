//! SQLite backend scaffold using libSQL.
//!
//! **Template stub — not a working storage backend.**
//! Construction and all operations fail closed so adopters cannot mistake
//! this for a production implementation. Replace the body with real libSQL
//! calls before enabling the `sqlite` feature in application code.
//! See: https://docs.rs/libsql

use crate::StorageError;
use std::future::Future;
use std::pin::Pin;

const STUB_MSG: &str = "SqliteBackend is a template stub (not implemented). Use MemoryBackend for tests, \
     or implement libSQL wiring before production use. See crates/hybrid-storage-template/README.md \
     and docs/patterns/trait-only-storage.md.";

/// SQLite backend scaffold (libSQL/Turso).
///
/// Intentionally incomplete: every method returns [`StorageError::Backend`]
/// so silent success is impossible.
pub struct SqliteBackend {
    _inner: Option<libsql::Database>,
}

impl SqliteBackend {
    /// Attempt to create a SQLite backend.
    ///
    /// Always returns [`Err`] until a real libSQL connection is implemented.
    ///
    /// The `url` parameter would be a libSQL connection URL, e.g.:
    /// - `"file::memory:?cache=shared"` for in-memory
    /// - `"libsql://your-db.turso.io"` for remote
    pub async fn new(_url: &str) -> Result<Self, StorageError> {
        Err(StorageError::Backend(STUB_MSG.into()))
    }

    fn not_implemented() -> StorageError {
        StorageError::Backend(STUB_MSG.into())
    }
}

impl crate::Backend for SqliteBackend {
    fn get(
        &self,
        _key: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<String>, StorageError>> + Send + '_>> {
        Box::pin(async { Err(Self::not_implemented()) })
    }

    fn set(
        &self,
        _key: &str,
        _value: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), StorageError>> + Send + '_>> {
        Box::pin(async { Err(Self::not_implemented()) })
    }

    fn delete(
        &self,
        _key: &str,
    ) -> Pin<Box<dyn Future<Output = Result<bool, StorageError>> + Send + '_>> {
        Box::pin(async { Err(Self::not_implemented()) })
    }

    fn list_keys(
        &self,
        _prefix: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, StorageError>> + Send + '_>> {
        Box::pin(async { Err(Self::not_implemented()) })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

    use super::*;

    #[tokio::test]
    async fn sqlite_backend_new_fails_closed() {
        let result = SqliteBackend::new("file::memory:?cache=shared").await;
        assert!(result.is_err(), "stub must not construct successfully");
        let msg = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(
            msg.contains("template stub") || msg.contains("not implemented"),
            "unexpected error: {msg}"
        );
    }
}
