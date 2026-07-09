//! # Sample Application
//!
//! A comprehensive sample application demonstrating the rust-2026-template features.
//!
//! ## Features demonstrated:
//! - Error handling with thiserror and anyhow
//! - Serialization with serde
//! - Logging with tracing
//! - CLI with clap

#![forbid(unsafe_code)]

/// Configuration types and loading logic.
pub mod config;
/// Item processing logic.
pub mod process;

pub use config::{
    AppError, Config, LogLevel, Result, init_logging, is_safe_char, load_config, sanitize_str,
};
pub use process::{Args, process_items};

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests;
