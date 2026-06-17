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
| `MigrationRegistry` | `register(from, to)` | Schema migration paths |
| `Storable` | `version()`, `migrate()` | Version-aware serialization trait |

## Architecture

- **`Storable`** — Trait for serializable state with version and migration support
- **`CheckpointManager<T>`** — Orchestrates save/load with atomic file operations
- **`CheckpointHeader`** — Metadata: version, timestamp, app name
- **`MigrationRegistry`** — Tracks allowed version transitions
- **`FileStorage`** — Atomic writes via temp file + rename (POSIX-safe)

## Features

- Atomic save operations (write to temp, rename on success)
- Version-aware deserialization with automatic migration hooks
- Schema evolution via `MigrationRegistry`
- Bincode serialization for compact binary checkpoints
