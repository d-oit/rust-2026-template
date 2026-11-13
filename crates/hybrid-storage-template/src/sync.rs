//! Incremental synchronization engine.

use super::{HybridStorage, PageCursor, StorageError, StorageOp};

/// Synchronization engine for replicating records between storage instances.
pub struct SyncEngine;

impl SyncEngine {
    /// Synchronize records incrementally from `source` to `target`.
    ///
    /// # Errors
    ///
    /// Propagates the errors documented on [`Self::sync_incremental_with_hook`].
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
    ///
    /// # Errors
    ///
    /// Returns any error from [`HybridStorage::get_sync_cursor`],
    /// [`HybridStorage::list_records_paginated`], [`Record::validate`],
    /// [`Backend::apply_transaction`], or the `on_page_synced` hook.
    ///
    /// [`Record::validate`]: crate::Record::validate
    /// [`Backend::apply_transaction`]: crate::Backend::apply_transaction
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
            let Some(last_rec) = page.records.last() else {
                break;
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

            // Commit first, then report: the hook is documented as running
            // "after each page commit". Firing it before
            // `apply_transaction` reported progress for pages that a failed
            // commit could still discard.
            target.primary.apply_transaction(&ops).await?;
            total_synced += page_count;
            page_num += 1;
            on_page_synced(page_num, &new_cursor)?;

            match page.next_cursor {
                Some(next) => current_cursor = Some(next),
                None => break,
            }
        }

        Ok(total_synced)
    }
}
