//! Entry point for the `cli_dry_run` example application.

use anyhow::{Context, Result};
use clap::Parser;
use cli_dry_run::{ApiClient, CliArgs, CliError, Plan, run_plan};

/// Production implementation of [`ApiClient`] that would call remote HTTP APIs.
struct RealApiClient;

impl ApiClient for RealApiClient {
    fn execute_remote(&self, step: usize, description: &str) -> Result<String, CliError> {
        // In a real application, this would make an outbound HTTP request.
        Ok(format!(
            "Remote API response for step {step}: successfully synthesized '{description}'"
        ))
    }
}

fn main() -> Result<()> {
    let args = CliArgs::parse();

    // 1. Validation & Planning (shared identical path for dry-run and live execution)
    let plan = Plan::build(&args).context("Failed to build execution plan")?;

    // 2. Client initialization
    let client = RealApiClient;

    // 3. Execution (or dry-run simulation)
    let report = run_plan(&plan, &client).context("Failed to run execution plan")?;

    // 4. Output results
    for line in report.log_output {
        println!("{line}");
    }

    Ok(())
}
