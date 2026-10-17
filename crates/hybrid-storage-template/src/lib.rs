//! # Hybrid Storage Template
//!
//! A template crate demonstrating hybrid storage patterns with state synchronization,
//! atomic transaction commits, monotonic modification watermarks, and keyset pagination.
//!
//! ## Consistency & Sync Guarantees
//!
//! - **Atomic Writes**: Record mutations and system modification watermarks are committed atomically via
//!   [`Backend::apply_transaction`]. Data and revision watermarks cannot diverge.
//! - **Bounded Keyset Pagination**: Queries are deterministically ordered by `(updated_at, id)` using `id`
//!   as a stable tie-breaker to prevent duplicate or skipped records across page boundaries.
//! - **Monotonic Watermarks**: Edits to older records update the modification watermark, ensuring changes
//!   are picked up by incremental synchronization.
//! - **Fail-Closed Validation**: Corrupt or missing required fields return explicit [`StorageError::CorruptRecord`]
//!   errors rather than default values.
//! - **Resumable Sync**: [`SyncEngine`] persists target high-watermark cursors after each page commit, enabling
//!   clean recovery from interrupted sync attempts.
//!
//! ## Limitations
//!
//! - In-memory locks in [`backends::MemoryBackend`] do not coordinate across multiple processes or distributed nodes.
//! - [`backends::SqliteBackend`] is an explicit fail-closed template stub requiring native libSQL transaction wiring before production use.
//! - Clock/watermark monotonicity assumes non-decreasing timestamps or sequence counters on write operations.

#![forbid(unsafe_code)]

pub mod backends;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use thiserror::Error;

/// Storage error types.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
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

    /// Corrupt record or missing required field.
    #[error("Corrupt record '{id}' (field '{field}'): {reason}")]
    CorruptRecord {
        /// Record ID if known.
        id: String,
        /// Field that is invalid or missing.
        field: String,
        /// Detail of why field is invalid.
        reason: String,
    },

    /// Transaction failed without committing.
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    /// Mutex poisoned (thread panicked while holding lock).
    #[error("Internal lock poisoned")]
    Poisoned,
}

/// A synchronized data record.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Record {
    /// Unique identifier for record.
    pub id: String,
    /// Record payload/content.
    pub data: String,
    /// Monotonic sequence or revision version.
    pub version: u64,
    /// Modification timestamp or watermark.
    pub updated_at: u64,
}

impl Record {
    /// Create a new record with initial values.
    pub fn new(
        id: impl Into<String>,
        data: impl Into<String>,
        version: u64,
        updated_at: u64,
    ) -> Self {
        Self {
            id: id.into(),
            data: data.into(),
            version,
            updated_at,
        }
    }

    /// Validate required fields fail-closed against corruption.
    pub fn validate(&self) -> Result<(), StorageError> {
        if self.id.trim().is_empty() {
            return Err(StorageError::CorruptRecord {
                id: self.id.clone(),
                field: "id".to_string(),
                reason: "record id cannot be empty".to_string(),
            });
        }
        if self.data.trim().is_empty() {
            return Err(StorageError::CorruptRecord {
                id: self.id.clone(),
                field: "data".to_string(),
                reason: "record data payload cannot be empty".to_string(),
            });
        }
        Ok(())
    }
}

/// Keyset pagination cursor using modification watermark and stable tie-breaker.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PageCursor {
    /// Last seen record modification watermark.
    pub updated_at: u64,
    /// Last seen record ID tie-breaker.
    pub id: String,
}

/// A page of query results for keyset pagination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// Records in current page.
    pub records: Vec<Record>,
    /// Cursor for fetching next page, if available.
    pub next_cursor: Option<PageCursor>,
}

/// An atomic storage operation within a transaction batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageOp {
    /// Set key-value pair.
    Set {
        /// Key to set.
        key: String,
        /// Value to store.
        value: String,
    },
    /// Delete key.
    Delete {
        /// Key to delete.
        key: String,
    },
}

/// Backend trait for storage implementations.
pub trait Backend: Send + Sync {
    /// Get a value by key.
    fn get(
        &self,
        key: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<String>, StorageError>> + Send + '_>>;

    /// Set a value by key.
    fn set(
        &self,
        key: &str,
        value: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), StorageError>> + Send + '_>>;

    /// Delete a value by key.
    fn delete(
        &self,
        key: &str,
    ) -> Pin<Box<dyn Future<Output = Result<bool, StorageError>> + Send + '_>>;

    /// List keys matching prefix.
    fn list_keys(
        &self,
        prefix: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, StorageError>> + Send + '_>>;

    /// Apply a sequence of operations atomically in a transaction batch.
    fn apply_transaction(
        &self,
        operations: &[StorageOp],
    ) -> Pin<Box<dyn Future<Output = Result<(), StorageError>> + Send + '_>>;
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

    /// Commit a record mutation and its modification watermark atomically.
    pub async fn commit_record_and_watermark(
        &self,
        record: &Record,
        watermark: u64,
    ) -> Result<(), StorageError> {
        record.validate()?;
        let json = serde_json::to_string(record)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        let ops = vec![
            StorageOp::Set {
                key: format!("rec:{}", record.id),
                value: json,
            },
            StorageOp::Set {
                key: "sys:watermark".to_string(),
                value: watermark.to_string(),
            },
        ];

        self.primary.apply_transaction(&ops).await
    }

    /// Get a record by ID with fail-closed field validation.
    pub async fn get_record(&self, id: &str) -> Result<Option<Record>, StorageError> {
        let key = format!("rec:{id}");
        let raw = match self.get(&key).await? {
            Some(v) => v,
            None => return Ok(None),
        };

        let record: Record =
            serde_json::from_str(&raw).map_err(|e| StorageError::CorruptRecord {
                id: id.to_string(),
                field: "payload".to_string(),
                reason: format!("malformed json payload: {e}"),
            })?;

        record.validate()?;
        Ok(Some(record))
    }

    /// Get current system modification watermark.
    pub async fn get_watermark(&self) -> Result<Option<u64>, StorageError> {
        let raw = match self.get("sys:watermark").await? {
            Some(v) => v,
            None => return Ok(None),
        };

        let watermark = raw
            .parse::<u64>()
            .map_err(|_| StorageError::CorruptRecord {
                id: "sys:watermark".to_string(),
                field: "watermark".to_string(),
                reason: "invalid non-numeric watermark format".to_string(),
            })?;

        Ok(Some(watermark))
    }

    /// Get target synchronization high-watermark cursor.
    pub async fn get_sync_cursor(&self) -> Result<Option<PageCursor>, StorageError> {
        let raw = match self.get("sys:sync_cursor").await? {
            Some(v) => v,
            None => return Ok(None),
        };

        let cursor: PageCursor =
            serde_json::from_str(&raw).map_err(|e| StorageError::CorruptRecord {
                id: "sys:sync_cursor".to_string(),
                field: "cursor".to_string(),
                reason: format!("invalid cursor json: {e}"),
            })?;

        Ok(Some(cursor))
    }

    /// List records using bounded keyset pagination with a stable `(updated_at, id)` tie-breaker.
    pub async fn list_records_paginated(
        &self,
        cursor: Option<&PageCursor>,
        limit: usize,
    ) -> Result<Page, StorageError> {
        let keys = self.primary.list_keys("rec:").await?;
        let mut all_records = Vec::with_capacity(keys.len());

        for key in keys {
            let id = key.trim_start_matches("rec:");
            if let Some(record) = self.get_record(id).await? {
                all_records.push(record);
            }
        }

        // Deterministic ordering by (updated_at ASC, id ASC)
        all_records.sort_by(|a, b| {
            a.updated_at
                .cmp(&b.updated_at)
                .then_with(|| a.id.cmp(&b.id))
        });

        // Apply cursor filtering: (updated_at > cursor.updated_at) OR (updated_at == cursor.updated_at AND id > cursor.id)
        let filtered: Vec<Record> = if let Some(c) = cursor {
            all_records
                .into_iter()
                .filter(|r| {
                    r.updated_at > c.updated_at || (r.updated_at == c.updated_at && r.id > c.id)
                })
                .collect()
        } else {
            all_records
        };

        if limit == 0 {
            return Ok(Page {
                records: Vec::new(),
                next_cursor: cursor.cloned(),
            });
        }

        let has_more = filtered.len() > limit;
        let page_records: Vec<Record> = filtered.into_iter().take(limit).collect();

        let next_cursor = if has_more {
            page_records.last().map(|r| PageCursor {
                updated_at: r.updated_at,
                id: r.id.clone(),
            })
        } else {
            None
        };

        Ok(Page {
            records: page_records,
            next_cursor,
        })
    }
}

/// Synchronization engine for replicating records between storage instances.
pub struct SyncEngine;

impl SyncEngine {
    /// Synchronize records incrementally from `source` to `target`.
    pub async fn sync_incremental(
        source: &HybridStorage,
        target: &HybridStorage,
        page_size: usize,
    ) -> Result<usize, StorageError> {
        Self::sync_incremental_with_hook(source, target, page_size, |_page_num, _cursor| Ok(()))
            .await
    }

    /// Synchronize records incrementally with a hook called after each page commit.
    ///
    /// The hook can be used for observing sync progress or simulating network failure/interruption.
    pub async fn sync_incremental_with_hook<H>(
        source: &HybridStorage,
        target: &HybridStorage,
        page_size: usize,
        mut on_page_synced: H,
    ) -> Result<usize, StorageError>
    where
        H: FnMut(usize, &PageCursor) -> Result<(), StorageError>,
    {
        let mut total_synced = 0;
        let mut current_cursor = target.get_sync_cursor().await?;
        let mut page_num = 0;

        loop {
            let page = source
                .list_records_paginated(current_cursor.as_ref(), page_size)
                .await?;

            if page.records.is_empty() {
                break;
            }

            let page_count = page.records.len();
            let last_rec = match page.records.last() {
                Some(r) => r,
                None => break,
            };

            let new_cursor = PageCursor {
                updated_at: last_rec.updated_at,
                id: last_rec.id.clone(),
            };

            let mut ops = Vec::with_capacity(page_count + 1);
            for rec in &page.records {
                rec.validate()?;
                let json = serde_json::to_string(rec)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                ops.push(StorageOp::Set {
                    key: format!("rec:{}", rec.id),
                    value: json,
                });
            }

            let cursor_json = serde_json::to_string(&new_cursor)
                .map_err(|e| StorageError::Serialization(e.to_string()))?;
            ops.push(StorageOp::Set {
                key: "sys:sync_cursor".to_string(),
                value: cursor_json,
            });

            page_num += 1;
            on_page_synced(page_num, &new_cursor)?;

            target.primary.apply_transaction(&ops).await?;
            total_synced += page_count;

            match page.next_cursor {
                Some(next) => current_cursor = Some(next),
                None => break,
            }
        }

        Ok(total_synced)
    }
}

impl Default for HybridStorage {
    fn default() -> Self {
        Self::new(Arc::new(backends::MemoryBackend::new()))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

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

    #[tokio::test]
    async fn test_transaction_failure_prevents_data_and_revision_divergence() {
        let memory = Arc::new(MemoryBackend::new());
        let storage = HybridStorage::new(memory.clone());

        let initial_rec = Record::new("rec1", "initial data", 1, 100);
        storage
            .commit_record_and_watermark(&initial_rec, 100)
            .await
            .unwrap();

        assert_eq!(storage.get_record("rec1").await.unwrap(), Some(initial_rec));
        assert_eq!(storage.get_watermark().await.unwrap(), Some(100));

        // Inject transaction failure
        memory.set_fail_transactions(true);

        let mutated_rec = Record::new("rec2", "uncommitted data", 2, 200);
        let result = storage.commit_record_and_watermark(&mutated_rec, 200).await;

        assert!(matches!(result, Err(StorageError::TransactionFailed(_))));

        // Verify neither new data nor revision watermark were updated
        assert_eq!(storage.get_record("rec2").await.unwrap(), None);
        assert_eq!(storage.get_watermark().await.unwrap(), Some(100));
    }

    #[tokio::test]
    async fn test_multipage_keyset_pagination_no_duplicates_or_skipped_records() {
        let storage = HybridStorage::default();

        // Create 10 records with varying timestamps and IDs
        for i in 1u64..=10u64 {
            let timestamp = (i % 3) * 10 + 100;
            let rec = Record::new(format!("item_{i:02}"), format!("payload_{i}"), 1, timestamp);
            storage
                .commit_record_and_watermark(&rec, timestamp)
                .await
                .unwrap();
        }

        let mut collected = Vec::new();
        let mut cursor = None;

        loop {
            let page = storage
                .list_records_paginated(cursor.as_ref(), 3)
                .await
                .unwrap();
            collected.extend(page.records);

            match page.next_cursor {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }

        assert_eq!(collected.len(), 10, "all 10 records must be retrieved");

        // Verify no duplicates
        let mut ids: Vec<String> = collected.iter().map(|r| r.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 10, "no duplicate records across page boundaries");

        // Verify ordering stability (updated_at ASC, id ASC)
        for window in collected.windows(2) {
            let (prev, next) = (&window[0], &window[1]);
            assert!(
                prev.updated_at < next.updated_at
                    || (prev.updated_at == next.updated_at && prev.id < next.id),
                "records must be strictly sorted by (updated_at, id)"
            );
        }
    }

    #[tokio::test]
    async fn test_edits_to_older_records_propagated_by_watermark() {
        let source = HybridStorage::default();
        let target = HybridStorage::default();

        let r1 = Record::new("r1", "v1", 1, 10);
        let r2 = Record::new("r2", "v1", 1, 20);
        let r3 = Record::new("r3", "v1", 1, 30);

        source.commit_record_and_watermark(&r1, 10).await.unwrap();
        source.commit_record_and_watermark(&r2, 20).await.unwrap();
        source.commit_record_and_watermark(&r3, 30).await.unwrap();

        // Initial sync copies r1, r2, r3 to target
        let synced = SyncEngine::sync_incremental(&source, &target, 2)
            .await
            .unwrap();
        assert_eq!(synced, 3);
        assert_eq!(target.get_record("r1").await.unwrap().unwrap().data, "v1");

        // Edit older record r1 with a newer modification watermark
        let r1_updated = Record::new("r1", "v2-edited", 2, 40);
        source
            .commit_record_and_watermark(&r1_updated, 40)
            .await
            .unwrap();

        // Incremental sync picks up edited r1
        let synced_after_edit = SyncEngine::sync_incremental(&source, &target, 2)
            .await
            .unwrap();
        assert_eq!(synced_after_edit, 1);

        let target_r1 = target.get_record("r1").await.unwrap().unwrap();
        assert_eq!(target_r1.data, "v2-edited");
        assert_eq!(target_r1.updated_at, 40);
    }

    #[tokio::test]
    async fn test_corrupt_required_fields_return_contextual_error() {
        let storage = HybridStorage::default();

        // 1. Record validation fails for empty id
        let invalid_id = Record::new("", "data", 1, 100);
        let err1 = storage.commit_record_and_watermark(&invalid_id, 100).await;
        assert!(matches!(
            err1,
            Err(StorageError::CorruptRecord { ref field, .. }) if field == "id"
        ));

        // 2. Record validation fails for empty data
        let invalid_data = Record::new("r1", "   ", 1, 100);
        let err2 = storage
            .commit_record_and_watermark(&invalid_data, 100)
            .await;
        assert!(matches!(
            err2,
            Err(StorageError::CorruptRecord { ref field, .. }) if field == "data"
        ));

        // 3. Stored malformed JSON payload returns explicit CorruptRecord
        storage
            .set("rec:corrupt_item", "{ invalid json string")
            .await
            .unwrap();
        let err3 = storage.get_record("corrupt_item").await;
        assert!(matches!(
            err3,
            Err(StorageError::CorruptRecord { ref id, ref field, .. })
                if id == "corrupt_item" && field == "payload"
        ));

        // 4. Stored non-numeric watermark returns explicit CorruptRecord
        storage.set("sys:watermark", "not_a_number").await.unwrap();
        let err4 = storage.get_watermark().await;
        assert!(matches!(
            err4,
            Err(StorageError::CorruptRecord { ref id, ref field, .. })
                if id == "sys:watermark" && field == "watermark"
        ));
    }

    #[tokio::test]
    async fn test_empty_data_equal_timestamps_and_interrupted_retried_sync() {
        let source = HybridStorage::default();
        let target = HybridStorage::default();

        // Empty data sync
        let empty_synced = SyncEngine::sync_incremental(&source, &target, 5)
            .await
            .unwrap();
        assert_eq!(empty_synced, 0);

        // Equal timestamps with stable tie-breaker pagination
        let timestamp = 1000;
        let ids = ["e", "b", "a", "d", "c"];
        for id in ids {
            let rec = Record::new(id, format!("data_{id}"), 1, timestamp);
            source
                .commit_record_and_watermark(&rec, timestamp)
                .await
                .unwrap();
        }

        let page1 = source.list_records_paginated(None, 2).await.unwrap();
        assert_eq!(page1.records.len(), 2);
        assert_eq!(page1.records[0].id, "a");
        assert_eq!(page1.records[1].id, "b");

        let page2 = source
            .list_records_paginated(page1.next_cursor.as_ref(), 2)
            .await
            .unwrap();
        assert_eq!(page2.records.len(), 2);
        assert_eq!(page2.records[0].id, "c");
        assert_eq!(page2.records[1].id, "d");

        let page3 = source
            .list_records_paginated(page2.next_cursor.as_ref(), 2)
            .await
            .unwrap();
        assert_eq!(page3.records.len(), 1);
        assert_eq!(page3.records[0].id, "e");
        assert!(page3.next_cursor.is_none());

        // Interrupted and retried sync
        // Add 2 more items (total 7 records, page_size 2 -> 4 pages)
        let rec_f = Record::new("f", "data_f", 1, 1001);
        let rec_g = Record::new("g", "data_g", 1, 1002);
        source
            .commit_record_and_watermark(&rec_f, 1001)
            .await
            .unwrap();
        source
            .commit_record_and_watermark(&rec_g, 1002)
            .await
            .unwrap();

        // Simulate network crash / interruption on page 2
        let sync_err =
            SyncEngine::sync_incremental_with_hook(&source, &target, 2, |page_num, _cursor| {
                if page_num == 2 {
                    Err(StorageError::Connection(
                        "simulated network drop".to_string(),
                    ))
                } else {
                    Ok(())
                }
            })
            .await;

        assert!(matches!(sync_err, Err(StorageError::Connection(_))));

        // Target should have page 1 (records "a" and "b") committed
        assert!(target.get_record("a").await.unwrap().is_some());
        assert!(target.get_record("b").await.unwrap().is_some());
        assert!(target.get_record("c").await.unwrap().is_none());

        // Retry sync without interruption
        let retried_count = SyncEngine::sync_incremental(&source, &target, 2)
            .await
            .unwrap();

        // 5 remaining records synced
        assert_eq!(retried_count, 5);

        // Verify target now contains all 7 records
        for id in ["a", "b", "c", "d", "e", "f", "g"] {
            assert!(
                target.get_record(id).await.unwrap().is_some(),
                "record {id} must be present in target after retry"
            );
        }
    }
}
