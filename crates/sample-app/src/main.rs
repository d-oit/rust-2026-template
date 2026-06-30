//! Sample application binary demonstrating the rust-2026-template.

use clap::Parser;
use std::io::Write;
use tracing::info;

use sample_app::{Args, init_logging, load_config, process_items};

/// Main entry point for the sample application.
fn main() -> sample_app::Result<()> {
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
