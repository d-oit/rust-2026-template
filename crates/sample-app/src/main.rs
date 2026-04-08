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
use std::io::Read;
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

/// Configuration for the application
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Name of the application
    pub app_name: String,
    /// Log level (trace, debug, info, warn, error)
    pub log_level: String,
    /// Maximum number of items to process
    pub max_items: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app_name: "sample-app".to_string(),
            log_level: "info".to_string(),
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

        // Capacity is safe to cast as we just checked against 1MB MAX_CONFIG_SIZE
        #[allow(clippy::cast_possible_truncation)]
        let mut contents = String::with_capacity(file_size as usize);
        file.take(MAX_CONFIG_SIZE).read_to_string(&mut contents)?;

        let config: Config = serde_json::from_str(&contents)?;
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
    for i in 1..=count {
        let mut s = String::with_capacity(9);
        let _ = write!(s, "item-{i:04}");
        items.push(s);
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
    println!("\nProcessed {} items:", items.len());
    for item in items.iter().take(5) {
        println!("  - {item}");
    }
    if items.len() > 5 {
        println!("  ... and {} more", items.len() - 5);
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
        assert_eq!(config.log_level, "info");
        assert_eq!(config.max_items, 100);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config {
            app_name: "test".to_string(),
            log_level: "debug".to_string(),
            max_items: 50,
        };

        let json = serde_json::to_string(&config).unwrap();
        let decoded: Config = serde_json::from_str(&json).unwrap();

        assert_eq!(config.app_name, decoded.app_name);
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
