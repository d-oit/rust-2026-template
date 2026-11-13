# Checkpoint Template

> Serializable application state with save/restore and version migration support.

## When to use

- Applications needing to persist and resume complex state across restarts
- Systems requiring schema evolution with backward-compatible migrations
- Scenarios where atomic file writes prevent corruption on crash

## Quick start

```rust,ignore
use checkpoint_template::{CheckpointManager, Storable};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct AppState {
    counter: u32,
    name: String,
}

impl Storable for AppState {
    fn version() -> u32 { 1 }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut manager = CheckpointManager::<AppState>::new("app_state.ckpt");

    // Save state
    let state = AppState { counter: 42, name: "demo".into() };
    manager.save(&state).await?;

    // Load state
    let loaded = manager.load().await?;
    println!("Loaded: {:?}", loaded);
    Ok(())
}
```

## Configuration

| Component | Options | Description |
|-----------|---------|-------------|
| `CheckpointManager<T>` | `new(path)` | Manages save/load for type `T` |
| `FileStorage` | `new(path)` | Atomic file backend (temp + rename) |
| `MigrationRegistry` | `register(from, to)`, `path()`, `migrate_with()` | Schema migration paths and chained steps |
| `Storable` | `version()`, `migrate()`, `supports_migration_from()` | Version-aware serialization trait |

## Architecture

- **`Storable`** — Trait for serializable state with version and migration support
- **`CheckpointManager<T>`** — Orchestrates save/load with atomic file operations
- **`CheckpointHeader`** — Metadata: version, timestamp, app name
- **`MigrationRegistry`** — Tracks allowed version transitions
- **`FileStorage`** — Atomic writes via temp file + rename (POSIX-safe)

## Features

- Atomic save operations (write to temp, rename on success)
- Version-aware deserialization with migration hooks that run on load
- Schema evolution via `MigrationRegistry` (chained multi-step paths)
- Bincode serialization for compact binary checkpoints

## Migrating an older checkpoint

Migrations operate on the stored **bincode bytes**, not JSON: bincode is not
self-describing, so a `serde_json::Value` payload could never be decoded.
Override `supports_migration_from` to opt in, and `migrate` to rewrite the
old layout:

```rust
# use serde::{Serialize, Deserialize};
# use checkpoint_template::{Storable, MigrationError};
#[derive(Debug, Clone, Serialize, Deserialize)]
struct V2 { value: u32, label: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
struct V1 { value: u32 }

impl Storable for V2 {
    fn version() -> u32 { 2 }

    fn supports_migration_from(from: u32) -> bool { from == 1 }

    fn migrate(payload: &[u8], from: u32) -> Result<Vec<u8>, MigrationError> {
        assert_eq!(from, 1);
        let cfg = bincode_reloaded::config::standard();
        let (old, _): (V1, usize) =
            bincode_reloaded::serde::decode_from_slice(payload, cfg)
                .map_err(|e| MigrationError::Corruption(e.to_string()))?;
        let upgraded = V2 { value: old.value, label: "migrated".into() };
        bincode_reloaded::serde::encode_to_vec(&upgraded, cfg)
            .map_err(|e| MigrationError::Corruption(e.to_string()))
    }
}
```

Without `supports_migration_from`, loading a different version fails closed with
`CheckpointError::VersionMismatch` rather than decoding an unknown layout.
