//! Sample application binary demonstrating the rust-2026-template.

use clap::Parser;
use std::io::Write;
use tracing::info;

use sample_app::{Args, init_logging, load_config, process_items};

/// Main entry point for the sample application.
fn main() -> sample_app::Result<()> {
    // Parse CLI arguments
    let args = Args::parse();

    // Load configuration first so `log_level` can seed the subscriber; log
    // output emitted before this point is discarded (no subscriber installed).
    let config = load_config(args.config)?;

    // Initialize logging from the configured level plus the `--verbose` override.
    init_logging(config.log_level, args.verbose);

    info!("Starting sample-app");
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
