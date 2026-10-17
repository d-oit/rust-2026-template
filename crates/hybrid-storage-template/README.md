# Hybrid Storage Template

> Feature-gated storage backends (SQL, KV, memory) with a unified trait abstraction.

## Which storage crate should I copy?

| Goal | Use |
|------|-----|
| Trait-only storage API + mock for tests | **`example-storage-pattern`** (start here for most apps) |
| Multi-backend wrapper + in-memory primary | **`hybrid-storage-template`** + `MemoryBackend` |
| Real SQLite/libSQL | Implement yourself — `SqliteBackend` is a **fail-closed stub** |

## When to use

- Applications needing pluggable storage backends (SQLite, redb, in-memory)
- Systems requiring a consistent API across different storage technologies
- Testing scenarios with in-memory backends that mirror production storage

## Quick start

```rust,ignore
use hybrid_storage_template::{HybridStorage, Backend, backends::MemoryBackend};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = HybridStorage::new(Arc::new(MemoryBackend::new()));

    storage.set("user:1", "Alice").await?;
    let value = storage.get("user:1").await?;
    println!("Got: {:?}", value);

    let keys = storage.list_keys("user:").await?;
    println!("User keys: {:?}", keys);

    storage.delete("user:1").await?;
    Ok(())
}
```

## Feature Flags

| Feature | Backend | Dependency | Default |
|---------|---------|------------|---------|
| — | `MemoryBackend` | None (always available) | — |
| `sqlite` | `SqliteBackend` (**stub**, fail-closed) | `libsql` | No |
| `kv` | `KvBackend` | `redb` | No |

```toml
[dependencies]
# Prefer MemoryBackend (no features). Only enable sqlite after implementing real wiring.
hybrid-storage-template = { path = "crates/hybrid-storage-template" }
```

## Architecture

- **`Backend`** — Async trait: `get`, `set`, `delete`, `list_keys`, `apply_transaction`
- **`HybridStorage`** — Wraps a primary `Arc<dyn Backend>`
- **`MemoryBackend`** — In-memory HashMap with atomic transaction support (working reference)
- **`SqliteBackend`** — Scaffold only; construction and ops return errors
- **`SyncEngine`** — Incremental synchronization engine between storage instances with cursor persistence

## Consistency & Synchronization Guarantees

- **Atomic Writes**: Record mutations and system modification watermarks are committed atomically in transaction batches (`apply_transaction`). Data and revision watermarks cannot diverge if a write fails.
- **Bounded Keyset Pagination**: Queries are deterministically ordered by `(updated_at, id)`. Using `id` as a stable tie-breaker prevents duplicate or skipped records across page boundaries, even when multiple records share identical timestamps.
- **Monotonic Modification Watermarks**: Updates to existing/older records update the modification watermark so incremental sync always captures edited records.
- **Fail-Closed Contextual Field Validation**: Records with missing or corrupt required fields (`id`, `data`) return explicit, contextual `StorageError::CorruptRecord` rather than fabricating default values.
- **Resumable Sync**: `SyncEngine` persists target high-watermark cursors after each page, allowing interrupted synchronization to resume without missing or re-processing records.

## Limitations

- **In-Memory Concurrency Scope**: `MemoryBackend` locks are single-process only and do not synchronize state across multiple processes or distributed nodes.
- **Fail-Closed SQLite Stub**: `SqliteBackend` is an explicit template stub that fails closed; production usage requires implementing native libSQL transactions.
- **Watermark Monotonicity**: Assumes client/system timestamps or sequence counters increase monotonically on writes.

## Testing

```rust,ignore
// Use MemoryBackend for fast, isolated tests
let storage = HybridStorage::default();
storage.set("key", "value").await.unwrap();
assert_eq!(storage.get("key").await.unwrap(), Some("value".into()));
```
