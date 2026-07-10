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

- **`Backend`** — Async trait: `get`, `set`, `delete`, `list_keys`
- **`HybridStorage`** — Wraps a primary `Arc<dyn Backend>`
- **`MemoryBackend`** — In-memory HashMap for testing (working reference)
- **`SqliteBackend`** — Scaffold only; construction and ops return errors
- **`KvBackend`** — redb persistent key-value store (requires `kv` feature)

## Testing

```rust,ignore
// Use MemoryBackend for fast, isolated tests
let storage = HybridStorage::default();
storage.set("key", "value").await.unwrap();
assert_eq!(storage.get("key").await.unwrap(), Some("value".into()));
```
