# Hybrid Storage Template

> Feature-gated storage backends (SQL, KV, memory) with a unified trait abstraction.

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
| `sqlite` | `SqliteBackend` | `libsql` | Yes |
| `kv` | `KvBackend` | `redb` | No |
| — | `MemoryBackend` | None (always available) | — |

```toml
[dependencies]
hybrid-storage-template = { features = ["sqlite", "kv"] }
```

## Architecture

- **`Backend`** — Async trait: `get`, `set`, `delete`, `list_keys`
- **`HybridStorage`** — Wraps a primary `Arc<dyn Backend>`
- **`MemoryBackend`** — In-memory HashMap for testing
- **`SqliteBackend`** — libSQL/Turso backend (requires `sqlite` feature)
- **`KvBackend`** — redb persistent key-value store (requires `kv` feature)

## Testing

```rust,ignore
// Use MemoryBackend for fast, isolated tests
let storage = HybridStorage::default();
storage.set("key", "value").await.unwrap();
assert_eq!(storage.get("key").await.unwrap(), Some("value".into()));
```
