//! # Sample Application
//!
//! A comprehensive sample application demonstrating the rust-2026-template features.
//!
//! ## Features demonstrated:
//! - Error handling with thiserror and anyhow
//! - Serialization with serde
//! - Logging with tracing
//! - CLI with clap

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
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

/// Returns true if the character is a safe, printable character.
///
/// This excludes standard control characters and Unicode bidirectional (Bidi)
/// control characters which can be used for log injection.
#[inline]
#[must_use]
pub const fn is_safe_char(c: char) -> bool {
    // Bolt: Hierarchical fast-path for character validation.
    let cp = c as u32;

    // Fast path: ASCII printable characters (0x20..=0x7E)
    if cp <= 0x7E {
        return cp >= 0x20;
    }

    // Fast path: ASCII/C1 control characters (0x00..=0x1F, 0x7F..=0x9F)
    if cp <= 0x9F {
        return false;
    }

    // Middle path: Latin-1, Basic Multilingual Plane characters below General Punctuation
    if cp < 0x2000 {
        // Exclude Soft Hyphen (0x00AD) and Arabic Letter Mark (0x061C)
        return cp != 0x00AD && cp != 0x061C;
    }

    // Punctuation and Format blocks (0x2000..=0x206F)
    if cp <= 0x206F {
        return !matches!(
            c,
            // Zero-width (200B..200D), Bidi control (200E..200F, 202A..202E, 2066..2069),
            // Line/Para separators (2028..2029), and Invisible formatters (2060..2064)
            '\u{200b}'..='\u{200f}' | '\u{2028}' | '\u{2029}' |
            '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{2064}' |
            '\u{2066}'..='\u{2069}'
        );
    }

    // Slow path: Common Unicode (CJK, Emojis, etc.) are safe except for Byte Order Mark
    cp != 0xFEFF
}

/// Sanitizes a string by replacing unsafe characters with '?'.
///
/// Returns a `Cow::Borrowed` if no changes were needed, avoiding unnecessary allocations.
#[must_use]
pub fn sanitize_str(s: &str) -> Cow<'_, str> {
    // Bolt: Fast path byte-scan for ASCII printable strings.
    // Skips UTF-8 decoding for the common case where the string is already clean.
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if !(0x20..=0x7E).contains(&b) {
            break;
        }
        i += 1;
    }

    if i == bytes.len() {
        return Cow::Borrowed(s);
    }

    // Slow path: character-by-character processing for Unicode or unsafe characters.
    let mut result = String::with_capacity(s.len());
    result.push_str(&s[..i]);

    // Bolt: Use .chars() instead of .char_indices() to avoid unused index overhead
    for c in s[i..].chars() {
        if is_safe_char(c) {
            result.push(c);
        } else {
            result.push('?');
        }
    }
    Cow::Owned(result)
}

/// Load configuration from file or use defaults
pub fn load_config(config_path: Option<PathBuf>) -> Result<Config> {
    // Security: Check file size before reading to prevent DoS (memory exhaustion)
    // Use a 1MB limit for configuration files
    const MAX_CONFIG_SIZE: u64 = 1024 * 1024;
    // Security: Limit max_items to prevent memory exhaustion during processing
    const MAX_ALLOWED_ITEMS: usize = 10000;
    // Security: Limit app_name length to prevent log-filling or resource exhaustion
    const MAX_APP_NAME_LEN: usize = 64;

    if let Some(path) = config_path {
        // Security: Sanitize path for logging and errors to prevent log injection.
        // Replace unsafe characters (control, Bidi) with '?' to keep logs safe.
        let path_str = path.to_string_lossy();
        let sanitized_path = sanitize_str(&path_str);

        info!("Loading config from: {sanitized_path}");

        // Security: Check metadata before opening to prevent hanging on FIFOs (DoS).
        let metadata = std::fs::metadata(&path)?;

        if !metadata.is_file() {
            return Err(AppError::Config(format!(
                "Config path is not a regular file: {sanitized_path}"
            )));
        }

        let file = std::fs::File::open(&path)?;
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
        for c in config.app_name.chars().filter(|c| is_safe_char(*c)) {
            // Security: Check if adding the next character would exceed the byte limit.
            // Strings are UTF-8, so characters can be up to 4 bytes.
            if sanitized_name.len() + c.len_utf8() > MAX_APP_NAME_LEN {
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
pub fn init_logging(verbose: bool) {
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

/// Lookup table for two-digit formatting to improve performance in hot loops.
static DIGITS_TABLE: [&str; 100] = [
    "00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15",
    "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26", "27", "28", "29", "30", "31",
    "32", "33", "34", "35", "36", "37", "38", "39", "40", "41", "42", "43", "44", "45", "46", "47",
    "48", "49", "50", "51", "52", "53", "54", "55", "56", "57", "58", "59", "60", "61", "62", "63",
    "64", "65", "66", "67", "68", "69", "70", "71", "72", "73", "74", "75", "76", "77", "78", "79",
    "80", "81", "82", "83", "84", "85", "86", "87", "88", "89", "90", "91", "92", "93", "94", "95",
    "96", "97", "98", "99",
];

/// Process items and return a result
pub fn process_items(count: usize, limit: usize) -> Result<Vec<String>> {
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

    // Bolt: Use nested loops to eliminate redundant division/remainder operations
    // when accessing the DIGITS_TABLE in the hot loop.
    'outer: for (tens, t_str) in DIGITS_TABLE.iter().enumerate() {
        for (ones, o_str) in DIGITS_TABLE.iter().enumerate() {
            let i = tens * 100 + ones;
            if i == 0 {
                continue; // continues inner loop; `break 'outer` below
            }
            if i > fast_count {
                break 'outer;
            }

            let mut s = String::with_capacity(9);
            s.push_str("item-");
            s.push_str(t_str);
            s.push_str(o_str);
            items.push(s);
        }
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
pub fn main() -> Result<()> {
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
mod tests;
