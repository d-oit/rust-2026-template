#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use super::*;
use tempfile::TempDir;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestState {
    value: u32,
}

impl Storable for TestState {
    fn version() -> u32 {
        1
    }
}

#[tokio::test]
async fn test_load_version_mismatch() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("version_mismatch.ckpt");

    let header = CheckpointHeader {
        version: 99, // Mismatched version
        created_at: SystemTime::UNIX_EPOCH,
        app_name: "test".to_string(),
    };
    let state = TestState { value: 42 };

    let config = bincode_reloaded::config::standard();
    let state_data = bincode_reloaded::serde::encode_to_vec(&state, config).unwrap();
    let mut combined = bincode_reloaded::serde::encode_to_vec(&header, config).unwrap();
    combined.extend_from_slice(&state_data);
    std::fs::write(&path, combined).unwrap();

    let manager = CheckpointManager::<TestState>::new(&path);
    let result = manager.load().await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, CheckpointError::VersionMismatch { .. }));
}

#[tokio::test]
async fn test_checkpoint_manager_save_load() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.ckpt");

    let mut manager = CheckpointManager::new(&path);
    let state = TestState { value: 42 };

    manager.save(&state).await.unwrap();
    let loaded = manager.load().await.unwrap();
    assert_eq!(loaded, Some(state));
}

#[tokio::test]
async fn test_checkpoint_manager_not_found() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("nonexistent.ckpt");

    let manager = CheckpointManager::<TestState>::new(&path);
    let result = manager.load().await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_load_config_too_large() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("too_large.ckpt");

    // Create a file larger than default 10MB
    let large_data = vec![0u8; 11 * 1024 * 1024];
    std::fs::write(&path, large_data).unwrap();

    let manager = CheckpointManager::<TestState>::new(&path);
    let result = manager.load().await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(
        err,
        CheckpointError::Storage(storage::StorageError::TooLarge(_))
    ));
}

#[tokio::test]
async fn test_load_config_invalid_type() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("directory.ckpt");
    std::fs::create_dir(&path).unwrap();

    let manager = CheckpointManager::<TestState>::new(&path);
    let result = manager.load().await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(
        err,
        CheckpointError::Storage(storage::StorageError::InvalidType)
    ));
}

#[tokio::test]
async fn test_load_config_app_name_too_long() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("long_app_name.ckpt");

    let header = CheckpointHeader {
        version: 1,
        created_at: SystemTime::UNIX_EPOCH,
        app_name: "a".repeat(257),
    };
    let state = TestState { value: 42 };

    let config = bincode_reloaded::config::standard();
    let state_data = bincode_reloaded::serde::encode_to_vec(&state, config).unwrap();

    let mut combined = bincode_reloaded::serde::encode_to_vec(&header, config).unwrap();
    combined.extend_from_slice(&state_data);

    std::fs::write(&path, combined).unwrap();

    let manager = CheckpointManager::<TestState>::new(&path);
    let result = manager.load().await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("app_name too long"));
}

#[tokio::test]
async fn test_load_config_app_name_control_chars() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("control_chars.ckpt");

    let header = CheckpointHeader {
        version: 1,
        created_at: SystemTime::UNIX_EPOCH,
        app_name: "test\napp".to_string(),
    };
    let state = TestState { value: 42 };

    let config = bincode_reloaded::config::standard();
    let state_data = bincode_reloaded::serde::encode_to_vec(&state, config).unwrap();

    let mut combined = bincode_reloaded::serde::encode_to_vec(&header, config).unwrap();
    combined.extend_from_slice(&state_data);

    std::fs::write(&path, combined).unwrap();

    let manager = CheckpointManager::<TestState>::new(&path);
    let result = manager.load().await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("control or Bidi characters"));
}

#[tokio::test]
async fn test_load_config_app_name_bidi_chars() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("bidi_chars.ckpt");

    let header = CheckpointHeader {
        version: 1,
        created_at: SystemTime::UNIX_EPOCH,
        // Use a Bidi control character (U+202A: LEFT-TO-RIGHT EMBEDDING)
        app_name: "test\u{202a}app".to_string(),
    };
    let state = TestState { value: 42 };

    let config = bincode_reloaded::config::standard();
    let state_data = bincode_reloaded::serde::encode_to_vec(&state, config).unwrap();

    let mut combined = bincode_reloaded::serde::encode_to_vec(&header, config).unwrap();
    combined.extend_from_slice(&state_data);

    std::fs::write(&path, combined).unwrap();

    let manager = CheckpointManager::<TestState>::new(&path);
    let result = manager.load().await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("control or Bidi characters"));
}

#[tokio::test]
async fn test_load_config_app_name_safe_unicode() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("safe_unicode.ckpt");

    let header = CheckpointHeader {
        version: 1,
        created_at: SystemTime::UNIX_EPOCH,
        app_name: "safe-🦀-app".to_string(),
    };
    let state = TestState { value: 42 };

    let config = bincode_reloaded::config::standard();
    let state_data = bincode_reloaded::serde::encode_to_vec(&state, config).unwrap();
    let mut combined = bincode_reloaded::serde::encode_to_vec(&header, config).unwrap();
    combined.extend_from_slice(&state_data);
    std::fs::write(&path, combined).unwrap();

    let manager = CheckpointManager::<TestState>::new(&path);
    let result = manager.load().await.unwrap();
    assert_eq!(result, Some(state));
}

#[tokio::test]
async fn test_with_custom_config() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("custom.ckpt");

    let config = CheckpointConfig {
        max_checkpoint_size: 100,
        max_app_name_len: 5,
    };

    let manager = CheckpointManager::<TestState>::with_config(&path, config);

    // App name too long for custom config
    let header = CheckpointHeader {
        version: 1,
        created_at: SystemTime::UNIX_EPOCH,
        app_name: "too_long".to_string(),
    };
    let state = TestState { value: 42 };

    let config = bincode_reloaded::config::standard();
    let state_data = bincode_reloaded::serde::encode_to_vec(&state, config).unwrap();
    let mut combined = bincode_reloaded::serde::encode_to_vec(&header, config).unwrap();
    combined.extend_from_slice(&state_data);
    std::fs::write(&path, combined).unwrap();

    let err = manager.load().await.unwrap_err().to_string();
    assert!(err.contains("app_name too long"));
}

#[tokio::test]
async fn test_save_too_large() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("save_too_large.ckpt");

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct LargeState {
        data: Vec<u8>,
    }
    impl Storable for LargeState {
        fn version() -> u32 {
            1
        }
    }

    let config = CheckpointConfig {
        max_checkpoint_size: 10,
        max_app_name_len: 256,
    };

    let mut manager = CheckpointManager::<LargeState>::with_config(&path, config);
    let state = LargeState {
        data: vec![0u8; 100],
    };

    let result = manager.save(&state).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("checkpoint too large"));
}

#[tokio::test]
async fn test_set_app_name_invalid() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("set_app_name.ckpt");

    let mut manager = CheckpointManager::<TestState>::new(&path);

    // Test length
    let result = manager.set_app_name("a".repeat(257));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("too long"));

    // Test control chars
    let result = manager.set_app_name("test\napp");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("control or Bidi characters")
    );

    // Test Bidi chars
    let result = manager.set_app_name("test\u{202a}app");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("control or Bidi characters")
    );

    // Test valid
    let result = manager.set_app_name("valid-app");
    assert!(result.is_ok());
}
