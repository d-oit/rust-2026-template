# Hybrid Storage Template

A template crate demonstrating hybrid storage with backend abstraction and SQL/KV implementations.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
hybrid-storage-template = { path = "../hybrid-storage-template", features = ["kv"] }
```

## Features

- **Backend trait**: Abstract storage interface (get, set, delete, list_keys)
- **MemoryBackend**: In-memory storage for testing
- **KvBackend**: Persistent KV store (requires `kv` feature)
- **SqliteBackend**: SQL backend (requires `sqlite` feature)

## Basic Setup

```rust
use hybrid_storage_template::{HybridStorage, backends::MemoryBackend};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let storage = HybridStorage::new(Arc::new(MemoryBackend::new()));

    // Set and get
    storage.set("key", "value").await?;
    let value = storage.get("key").await?;

    // List with prefix
    let keys = storage.list_keys("user:").await?;

    // Delete
    storage.delete("key").await?;

    Ok(())
}
```

## Feature Flags

- `kv` - Enable KV backend support
- `sqlite` - Enable SQLite backend support