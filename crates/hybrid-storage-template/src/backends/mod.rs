//! Storage backend implementations.

#[cfg(feature = "sqlite")]
pub mod sqlite;

use crate::StorageError;

/// Memory backend for testing.
pub struct MemoryBackend {
    data: std::sync::Mutex<std::collections::HashMap<String, String>>,
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
        }
    }
}

#[async_trait::async_trait]
impl crate::Backend for MemoryBackend {
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

#[cfg(test)]
mod tests {
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
