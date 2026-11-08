use clap::Parser;
use tracing::{error, info, warn};

use crate::config::{AppError, Result};

/// CLI arguments
#[derive(Parser, Debug)]
#[command(name = "sample-app")]
#[command(version, about = "A sample application using rust-2026-template", long_about = None)]
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

/// Process `count` items, returning one `item-NNNN` label per item.
///
/// # Errors
///
/// Returns [`AppError::Config`] if `count` exceeds `limit`.
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

    // Bolt: Use nested loops over a precomputed two-digit table to avoid
    // division/remainder per item. Items are `item-NNNN`, numbered from 1 and
    // zero-padded to four digits; the thousands group is formatted directly so
    // the loop covers every count rather than stopping at an arbitrary bound.
    'outer: for (thousands, prefix) in ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"]
        .iter()
        .enumerate()
    {
        for (tens, t_str) in DIGITS_TABLE.iter().enumerate() {
            for (ones, o_str) in DIGITS_TABLE.iter().enumerate() {
                let i = thousands * 10_000 + tens * 100 + ones;
                if i == 0 {
                    continue; // Item numbering starts at 1.
                }
                if i > count {
                    break 'outer;
                }

                let mut s = String::with_capacity("item-".len() + 4);
                s.push_str("item-");
                if thousands > 0 {
                    s.push_str(prefix);
                }
                s.push_str(t_str);
                s.push_str(o_str);
                items.push(s);
            }
        }
    }

    debug_assert_eq!(
        items.len(),
        count,
        "process_items must return exactly `count` items"
    );

    info!("Successfully processed {} items", items.len());
    Ok(items)
}
