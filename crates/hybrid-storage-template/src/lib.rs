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
#[path = "tests.rs"]
mod tests;
