//! Storage backend implementations.

#[cfg(feature = "sqlite")]
pub mod sqlite;

use crate::{StorageError, StorageOp};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};

/// Memory backend for testing.
pub struct MemoryBackend {
    data: std::sync::Mutex<std::collections::HashMap<String, String>>,
    fail_transactions: AtomicBool,
}

impl Default for MemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryBackend {
    /// Create a new memory backend.
    pub fn new() -> Self {
        Self {
            data: std::sync::Mutex::new(std::collections::HashMap::new()),
            fail_transactions: AtomicBool::new(false),
        }
    }

    /// Set simulated transaction failure flag for testing atomicity.
    pub fn set_fail_transactions(&self, fail: bool) {
        self.fail_transactions.store(fail, Ordering::SeqCst);
    }
}

impl crate::Backend for MemoryBackend {
    fn get(
        &self,
        key: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<String>, StorageError>> + Send + '_>> {
        let key = key.to_string();
        Box::pin(async move {
            Ok(self
                .data
                .lock()
                .map_err(|_| StorageError::Poisoned)?
                .get(&key)
                .cloned())
        })
    }

    fn set(
        &self,
        key: &str,
        value: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), StorageError>> + Send + '_>> {
        let key = key.to_string();
        let value = value.to_string();
        Box::pin(async move {
            self.data
                .lock()
                .map_err(|_| StorageError::Poisoned)?
                .insert(key, value);
            Ok(())
        })
    }

    fn delete(
        &self,
        key: &str,
    ) -> Pin<Box<dyn Future<Output = Result<bool, StorageError>> + Send + '_>> {
        let key = key.to_string();
        Box::pin(async move {
            Ok(self
                .data
                .lock()
                .map_err(|_| StorageError::Poisoned)?
                .remove(&key)
                .is_some())
        })
    }

    fn list_keys(
        &self,
        prefix: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, StorageError>> + Send + '_>> {
        let prefix = prefix.to_string();
        Box::pin(async move {
            Ok(self
                .data
                .lock()
                .map_err(|_| StorageError::Poisoned)?
                .keys()
                .filter(|k| k.starts_with(&prefix))
                .cloned()
                .collect())
        })
    }

    fn apply_transaction(
        &self,
        operations: &[StorageOp],
    ) -> Pin<Box<dyn Future<Output = Result<(), StorageError>> + Send + '_>> {
        let operations = operations.to_vec();
        Box::pin(async move {
            if self.fail_transactions.load(Ordering::SeqCst) {
                return Err(StorageError::TransactionFailed(
                    "simulated atomic commit transaction failure".to_string(),
                ));
            }
            let mut data = self.data.lock().map_err(|_| StorageError::Poisoned)?;
            for op in operations {
                match op {
                    StorageOp::Set { key, value } => {
                        data.insert(key, value);
                    }
                    StorageOp::Delete { key } => {
                        data.remove(&key);
                    }
                }
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

    use super::*;
    use crate::Backend;

    #[tokio::test]
    async fn test_list_keys() {
        let backend = MemoryBackend::new();
        backend.set("user:1", "a").await.unwrap();
        backend.set("user:2", "b").await.unwrap();
        backend.set("config:x", "c").await.unwrap();

        let mut keys = backend.list_keys("user:").await.unwrap();
        keys.sort();
        assert_eq!(keys, vec!["user:1", "user:2"]);
    }
}
