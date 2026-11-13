#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    missing_docs,
    reason = "test modules assert on known-good fixtures, so panicking macros are the intended failure signal"
)]

//! Migration regression tests.

use super::*;
use tempfile::TempDir;

/// A v2 state that knows how to read the v1 layout.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct MigratedState {
    value: u32,
    label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MigratedStateV1 {
    value: u32,
}

impl Storable for MigratedState {
    fn version() -> u32 {
        2
    }

    fn supports_migration_from(from_version: u32) -> bool {
        from_version == 1
    }

    fn migrate(payload: &[u8], from_version: u32) -> Result<Vec<u8>, MigrationError> {
        assert_eq!(from_version, 1);
        let config = bincode_reloaded::config::standard();
        let (old, _): (MigratedStateV1, usize) =
            bincode_reloaded::serde::decode_from_slice(payload, config)
                .map_err(|e| MigrationError::Corruption(e.to_string()))?;
        let upgraded = Self {
            value: old.value,
            label: "migrated".to_string(),
        };
        bincode_reloaded::serde::encode_to_vec(&upgraded, config)
            .map_err(|e| MigrationError::Corruption(e.to_string()))
    }
}

fn write_checkpoint(path: &Path, version: u32, payload: &[u8]) {
    let header = CheckpointHeader {
        version,
        created_at: SystemTime::UNIX_EPOCH,
        app_name: "test".to_string(),
    };
    let config = bincode_reloaded::config::standard();
    let mut combined = bincode_reloaded::serde::encode_to_vec(&header, config).unwrap();
    combined.extend_from_slice(payload);
    std::fs::write(path, combined).unwrap();
}

#[tokio::test]
async fn test_load_runs_registered_migration() {
    // Regression: `load` returned VersionMismatch and never called
    // `Storable::migrate`, so the documented migration support did not exist.
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("migrate.ckpt");

    let config = bincode_reloaded::config::standard();
    let old_payload =
        bincode_reloaded::serde::encode_to_vec(&MigratedStateV1 { value: 7 }, config).unwrap();
    write_checkpoint(&path, 1, &old_payload);

    let manager = CheckpointManager::<MigratedState>::new(&path);
    let loaded = manager.load().await.unwrap();

    assert_eq!(
        loaded,
        Some(MigratedState {
            value: 7,
            label: "migrated".to_string()
        }),
        "v1 checkpoint must be migrated to v2 on load"
    );
}

/// A v2 type that deliberately does **not** declare migration support.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct NoMigrationState {
    value: u32,
}

impl Storable for NoMigrationState {
    fn version() -> u32 {
        2
    }
}

#[tokio::test]
async fn test_load_without_migration_opt_in_fails_closed() {
    // A type that does not declare support must not silently decode foreign bytes.
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("no_opt_in.ckpt");

    let config = bincode_reloaded::config::standard();
    let payload =
        bincode_reloaded::serde::encode_to_vec(&MigratedStateV1 { value: 1 }, config).unwrap();
    write_checkpoint(&path, 1, &payload);

    let manager = CheckpointManager::<NoMigrationState>::new(&path);
    let err = manager.load().await.unwrap_err();
    assert!(matches!(err, CheckpointError::VersionMismatch { .. }));
}
