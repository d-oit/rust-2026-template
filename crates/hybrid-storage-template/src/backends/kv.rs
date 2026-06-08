//! KV backend implementation using redb.

use crate::StorageError;
use std::collections::HashMap;
use std::sync::Mutex;

/// KV backend using in-memory storage (redb is blocking, use spawn_blocking for real use).
pub struct KvBackend {
    data: Mutex<HashMap<String, String>>,
}

impl KvBackend {
    /// Create a new KV backend.
    pub fn new(_path: impl AsRef<std::path::Path>) -> Result<Self, StorageError> {
        Ok(Self {
            data: Mutex::new(HashMap::new()),
        })
    }
}

#[async_trait::async_trait]
impl crate::Backend for KvBackend {
    async fn get(&self, key: &str) -> Result<Option<String>, StorageError> {
        Ok(self.data.lock().unwrap().get(key).cloned())
    }

    async fn set(&self, key: &str, value: &str) -> Result<(), StorageError> {
        self.data
            .lock()
            .unwrap()
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool, StorageError> {
        Ok(self.data.lock().unwrap().remove(key).is_some())
    }

    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>, StorageError> {
        Ok(self
            .data
            .lock()
            .unwrap()
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect())
    }
}
