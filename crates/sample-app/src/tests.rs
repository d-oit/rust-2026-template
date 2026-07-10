#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use crate::{Args, Config, LogLevel, is_safe_char, load_config, process_items, sanitize_str};
use clap::Parser;
use std::borrow::Cow;

#[test]
fn test_config_default() {
    let config = Config::default();
    assert_eq!(config.app_name, "sample-app");
    assert_eq!(config.log_level, LogLevel::Info);
    assert_eq!(config.max_items, 100);
}

#[test]
fn test_config_serialization() {
    let config = Config {
        app_name: "test".to_string(),
        log_level: LogLevel::Debug,
        max_items: 50,
    };

    let json = serde_json::to_string(&config).unwrap();
    let decoded: Config = serde_json::from_str(&json).unwrap();

    assert_eq!(config.app_name, decoded.app_name);
    assert_eq!(config.log_level, decoded.log_level);
}

#[test]
fn test_config_invalid_log_level() {
    let json = r#"{
        "app_name": "test",
        "log_level": "invalid",
        "max_items": 100
    }"#;

    let result: std::result::Result<Config, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn test_config_recursion_limit() {
    let mut json = String::from("1");
    for _ in 0..150 {
        json = format!("[{json}]");
    }

    let result: std::result::Result<serde_json::Value, _> = serde_json::from_str(&json);

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("recursion limit exceeded"));
}

#[test]
fn test_is_safe_char_printable() {
    assert!(is_safe_char('a'));
    assert!(is_safe_char('1'));
    assert!(is_safe_char(' '));
    assert!(is_safe_char('🦀'));
    assert!(is_safe_char('ü'));
}

#[test]
fn test_is_safe_char_unprintable() {
    assert!(!is_safe_char('\n'));
    assert!(!is_safe_char('\r'));
    assert!(!is_safe_char('\t'));
    assert!(!is_safe_char('\u{200e}'));
    assert!(!is_safe_char('\u{202e}'));
    assert!(!is_safe_char('\u{2066}'));
    assert!(!is_safe_char('\u{200b}'));
    assert!(!is_safe_char('\u{200c}'));
    assert!(!is_safe_char('\u{200d}'));
    assert!(!is_safe_char('\u{2060}'));
    assert!(!is_safe_char('\u{feff}'));
    assert!(!is_safe_char('\u{00ad}'));
}

#[test]
fn test_config_app_name_sanitization() {
    let name = String::from("test\napp\u{202e}r");
    let sanitized: String = name.chars().filter(|c| is_safe_char(*c)).collect();
    assert_eq!(sanitized, "testappr");
}

#[test]
fn test_load_config_from_directory() {
    let result = load_config(Some(std::path::PathBuf::from(".")));
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("not a regular file")
    );
}

#[test]
fn test_load_config_app_name_too_long() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("too_long_config.json");
    let json = r#"{
        "app_name": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "log_level": "info",
        "max_items": 100
    }"#;
    std::fs::write(&file_path, json).unwrap();

    let result = load_config(Some(file_path));
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("app_name too long")
    );
}

#[test]
fn test_load_config_app_name_multibyte_too_long() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("multibyte_too_long_config.json");
    let app_name = format!("{}🦀", "a".repeat(63));
    let json = format!(
        r#"{{
        "app_name": "{app_name}",
        "log_level": "info",
        "max_items": 100
    }}"#
    );
    std::fs::write(&file_path, json).unwrap();

    let result = load_config(Some(file_path));
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("app_name too long")
    );
}

#[test]
fn test_config_deny_unknown_fields() {
    let json = r#"{
        "app_name": "test",
        "log_level": "info",
        "max_items": 100,
        "unknown_field": "oops"
    }"#;

    let result: std::result::Result<Config, _> = serde_json::from_str(json);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("unknown field `unknown_field`"));
}

#[test]
fn test_process_items_zero() {
    let result = process_items(0, 100).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_process_items_normal() {
    let result = process_items(5, 100).unwrap();
    assert_eq!(result.len(), 5);
    assert_eq!(result[0], "item-0001");
}

#[test]
fn test_process_items_too_many() {
    let result = process_items(101, 100);
    assert!(result.is_err());
}

#[test]
fn test_args_parsing() {
    let args = Args::parse_from(["sample-app", "--count", "5"]);
    assert_eq!(args.count, 5);
}

#[test]
fn test_sanitize_str_clean_ascii() {
    let input = "hello world 123";
    let result = sanitize_str(input);
    assert!(matches!(result, Cow::Borrowed(_)));
    assert_eq!(&*result, "hello world 123");
}

#[test]
fn test_sanitize_str_unsafe_chars() {
    let input = "hello\nworld\t!";
    let result = sanitize_str(input);
    assert!(matches!(result, Cow::Owned(_)));
    assert_eq!(&*result, "hello?world?!");
}

#[test]
fn test_sanitize_str_bidi_override() {
    let input = "safe\u{202e}injected";
    let result = sanitize_str(input);
    assert_eq!(&*result, "safe?injected");
}

#[test]
fn test_sanitize_str_multibyte_unicode() {
    let input = "hello \u{1F980} world";
    let result = sanitize_str(input);
    assert_eq!(&*result, "hello \u{1F980} world");
}

#[test]
fn test_sanitize_str_empty() {
    let input = "";
    let result = sanitize_str(input);
    assert!(matches!(result, Cow::Borrowed(_)));
    assert_eq!(&*result, "");
}

#[test]
fn test_sanitize_str_starts_unsafe() {
    let input = "\nhello";
    let result = sanitize_str(input);
    assert_eq!(&*result, "?hello");
}

#[test]
fn test_sanitize_str_all_unsafe() {
    let input = "\n\t\r";
    let result = sanitize_str(input);
    assert_eq!(&*result, "???");
}

#[test]
fn test_sanitize_str_format_chars() {
    let input = "a\u{200b}b\u{2060}c";
    let result = sanitize_str(input);
    assert_eq!(&*result, "a?b?c");
}
