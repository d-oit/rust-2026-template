# Checkpoint Template

A template crate demonstrating checkpoint-based state persistence with versioning and migration support.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
checkpoint-template = { path = "../checkpoint-template" }
```

## Basic Setup

```rust
use checkpoint_template::{CheckpointManager, Storable, FileStorage};

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct MyState {
    value: u32,
}

impl Storable for MyState {
    type Error = checkpoint_template::StorageError;

    async fn save(&self, manager: &CheckpointManager) -> Result<(), Self::Error> {
        manager.save(self).await
    }

    async fn load(manager: &CheckpointManager) -> Result<Self, Self::Error> {
        manager.load().await
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let storage = FileStorage::new("checkpoint.ckpt");
    let manager = CheckpointManager::new(storage);

    // Save state
    let state = MyState::default();
    state.save(&manager).await?;

    // Load state
    let loaded = MyState::load(&manager).await?;
    println!("Loaded value: {}", loaded.value);

    Ok(())
}
```

## Features

- **CheckpointHeader**: Version, timestamp, and app name metadata
- **MigrationRegistry**: Schema migration between versions
- **FileStorage**: Atomic writes via temp file rename
- **Storable trait**: Implement for any serializable type