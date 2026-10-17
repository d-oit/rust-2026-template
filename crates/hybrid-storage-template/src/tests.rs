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
