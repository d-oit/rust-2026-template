//! # Sample Application
//!
//! A comprehensive sample application demonstrating the rust-2026-template features.
//!
//! ## Features demonstrated:
//! - Async runtime with tokio
//! - Error handling with thiserror and anyhow
//! - Serialization with serde
//! - Logging with tracing
//! - CLI with clap

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use thiserror::Error;
use tracing::{error, info, warn};

/// Custom error types for the application
#[derive(Error, Debug)]
pub enum AppError {
    /// IO error from file system operations
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
}

/// Result type alias using our custom error
pub type Result<T> = std::result::Result<T, AppError>;

/// Supported log levels for the application
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Finest-grained informational events
    Trace,
    /// Fine-grained informational events that are most useful to debug an application
    Debug,
    /// Informational messages that highlight the progress of the application
    Info,
    /// Potentially harmful situations
    Warn,
    /// Error events that might still allow the application to continue running
    Error,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
    }
}

/// Configuration for the application
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Name of the application
    pub app_name: String,
    /// Log level for the application
    pub log_level: LogLevel,
    /// Maximum number of items to process
    pub max_items: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app_name: "sample-app".to_string(),
            log_level: LogLevel::default(),
            max_items: 100,
        }
    }
}

/// CLI arguments
#[derive(Parser, Debug)]
#[command(name = "sample-app")]
#[command(about = "A sample application using rust-2026-template", long_about = None)]
struct Args {
    /// Path to config file (optional)
    #[arg(long)]
    config: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Number of items to process
    #[arg(short, long, default_value_t = 10)]
    count: usize,
}

/// Load configuration from file or use defaults
fn load_config(config_path: Option<PathBuf>) -> Result<Config> {
    // Security: Check file size before reading to prevent DoS (memory exhaustion)
    // Use a 1MB limit for configuration files
    const MAX_CONFIG_SIZE: u64 = 1024 * 1024;
    // Security: Limit max_items to prevent memory exhaustion during processing
    const MAX_ALLOWED_ITEMS: usize = 10000;
    // Security: Limit app_name length to prevent log-filling or resource exhaustion
    const MAX_APP_NAME_LEN: usize = 64;

    if let Some(path) = config_path {
        info!("Loading config from: {}", path.display());

        let file = std::fs::File::open(&path)?;
        let metadata = file.metadata()?;

        if !metadata.is_file() {
            return Err(AppError::Config(format!(
                "Config path is not a regular file: {}",
                path.display()
            )));
        }

        let file_size = metadata.len();
        if file_size > MAX_CONFIG_SIZE {
            return Err(AppError::Config(format!(
                "Config file too large: {file_size} bytes (max {MAX_CONFIG_SIZE})"
            )));
        }

        let reader = BufReader::new(file.take(MAX_CONFIG_SIZE));

        // Security: serde_json has a default recursion limit of 128 which
        // provides protection against stack overflow DoS.
        let mut config: Config = serde_json::from_reader(reader)?;

        // Security: Sanitize app_name and check length to prevent log injection and resource exhaustion.
        // We check length during sanitization to avoid unnecessary allocations.
        let mut sanitized_name = String::with_capacity(config.app_name.len().min(MAX_APP_NAME_LEN));
        for c in config.app_name.chars().filter(|c| !c.is_control()) {
            if sanitized_name.len() >= MAX_APP_NAME_LEN {
                return Err(AppError::Config(format!(
                    "app_name too long: exceeds maximum of {MAX_APP_NAME_LEN} bytes"
                )));
            }
            sanitized_name.push(c);
        }
        config.app_name = sanitized_name;

        // Security: Validate max_items to prevent OOM
        if config.max_items > MAX_ALLOWED_ITEMS {
            return Err(AppError::Config(format!(
                "max_items {} exceeds limit of {MAX_ALLOWED_ITEMS}",
                config.max_items
            )));
        }

        Ok(config)
    } else {
        info!("Using default configuration");
        Ok(Config::default())
    }
}

/// Initialize logging with tracing
fn init_logging(verbose: bool) {
    let level = if verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        // Disable thread metadata for CLI performance
        .with_thread_ids(false)
        .with_thread_names(false)
        .init();
}

/// Process items and return a result
fn process_items(count: usize, limit: usize) -> Result<Vec<String>> {
    info!("Processing {} items (limit: {})", count, limit);

    if count == 0 {
        warn!("No items to process");
        return Ok(vec![]);
    }

    if count > limit {
        error!("Too many items requested: {count} (limit: {limit})");
        return Err(AppError::Config(format!(
            "Cannot process more than {limit} items, got {count}"
        )));
    }

    // Pre-allocate Vec and Strings for efficiency
    let mut items = Vec::with_capacity(count);

    // Bolt: Split loop to remove branch and dynamic capacity check from hot loop
    let fast_count = count.min(9999);
    for i in 1..=fast_count {
        let mut s = String::with_capacity(9);
        s.push_str("item-");

        // Bolt: Optimized digit extraction (4 digits)
        let q100 = i / 100;
        let q10 = i / 10;
        #[allow(clippy::cast_possible_truncation)]
        {
            s.push((b'0' + (q100 / 10) as u8) as char);
            s.push((b'0' + (q100 % 10) as u8) as char);
            s.push((b'0' + (q10 % 10) as u8) as char);
            s.push((b'0' + (i % 10) as u8) as char);
        }
        items.push(s);
    }

    // Bolt: Handle boundary cases separately
    if count >= 10000 {
        let mut s = String::with_capacity(10);
        s.push_str("item-10000");
        items.push(s);

        // Fallback for counts > 10000 if limit increases
        for i in 10001..=count {
            let mut s = String::with_capacity(10);
            s.push_str("item-");
            let _ = write!(s, "{i}");
            items.push(s);
        }
    }

    info!("Successfully processed {} items", items.len());
    Ok(items)
}

/// Main application entry point
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let args = Args::parse();

    // Initialize logging
    init_logging(args.verbose);

    info!("Starting sample-app");

    // Load configuration
    let config = load_config(args.config)?;
    info!("App name: {}", config.app_name);

    // Process items
    let items = process_items(args.count, config.max_items)?;

    // Print results
    // Bolt: Lock stdout to minimize locking overhead and syscalls for multiple prints
    {
        use std::io::Write;
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        let _ = writeln!(handle, "\nProcessed {} items:", items.len());
        for item in items.iter().take(5) {
            let _ = writeln!(handle, "  - {item}");
        }
        if items.len() > 5 {
            let _ = writeln!(handle, "  ... and {} more", items.len() - 5);
        }
        let _ = handle.flush();
    }

    info!("Application completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // Create a deeply nested JSON that exceeds serde_json's default limit of 128
        let mut json = String::from("1");
        for _ in 0..150 {
            json = format!("[{json}]");
        }

        // We use serde_json::Value to test the recursion limit specifically
        let result: std::result::Result<serde_json::Value, _> = serde_json::from_str(&json);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("recursion limit exceeded"));
    }

    #[test]
    fn test_config_app_name_sanitization() {
        // Let's just verify the sanitization logic
        let name = String::from("test\napp\r");
        let sanitized: String = name.chars().filter(|c| !c.is_control()).collect();
        assert_eq!(sanitized, "testapp");
    }

    #[test]
    fn test_load_config_app_name_too_long() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("too_long_config.json");
        let json = r#"{
            "app_name": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "log_level": "info",
            "max_items": 100
        }"#;
        std::fs::write(&file_path, json).unwrap();

        let result = load_config(Some(file_path.clone()));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("app_name too long")
        );

        let _ = std::fs::remove_file(file_path);
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
}
