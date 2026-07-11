use clap::Parser;
use tracing::{error, info, warn};

use crate::config::{AppError, Result};

/// CLI arguments
#[derive(Parser, Debug)]
#[command(name = "sample-app")]
#[command(about = "A sample application using rust-2026-template", long_about = None)]
pub struct Args {
    /// Path to config file (optional)
    #[arg(long)]
    pub config: Option<std::path::PathBuf>,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,

    /// Number of items to process
    #[arg(short, long, default_value_t = 10)]
    pub count: usize,
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

///
/// # Errors
///
/// Returns `AppError::Config` if `count` exceeds `limit`.
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

    // Handle boundary case: count == 10000 (the max allowed by load_config)
    if count >= 10000 {
        items.push("item-10000".to_string());
    }

    info!("Successfully processed {} items", items.len());
    Ok(items)
}
