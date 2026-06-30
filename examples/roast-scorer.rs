#![allow(missing_docs)]

//! Roast Scorer example — runs `scripts/roast-scorer.sh` via `bash`.
//!
//! Demonstrates proper `?`-propagation from a `Result<(), Box<dyn Error>>`-returning
//! `main`, paired with the workspace's `-D warnings` policy, so clippy's
//! `expect_used` and `missing_docs` lints stay silent without runtime
//! `.expect(...)` calls. If `scripts/roast-scorer.sh` is absent from the
//! checkout, `Command::status()` returns the underlying spawn error and the
//! example fails honestly with a non-zero exit.

use std::process::Command;

/// Entry point: invokes the roast-scorer shell script and propagates errors
/// via `?` rather than `.expect()`. Errors in spawning or the script's own
/// non-zero exit are returned through `main`'s `Result`.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let script = "scripts/roast-scorer.sh";
    let status = Command::new("bash")
        .arg(script)
        .status()
        .map_err(|e| format!("failed to spawn bash for {script}: {e}"))?;

    if !status.success() {
        return Err(format!("script exited with status {status}").into());
    }

    Ok(())
}
