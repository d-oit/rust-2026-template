//! xtask — thin wrappers around template quality tooling.
//!
//! **Canonical full gate:** `./scripts/quality-gates.sh`
//!
//! ```bash
//! cargo run -p xtask quality-gates   # delegates to scripts/quality-gates.sh
//! cargo run -p xtask fmt             # Format check only
//! cargo run -p xtask clippy          # Clippy with -D warnings
//! cargo run -p xtask deny            # cargo deny check
//! cargo run -p xtask test            # Run tests (nextest if available)
//! ```
#![forbid(unsafe_code)]
#![allow(clippy::print_stdout, clippy::print_stderr)]

use clap::{Parser, Subcommand};
use std::process::Command;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Quality helpers; full gate is scripts/quality-gates.sh")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run the full quality gate script (SSOT for pre-push checks)
    QualityGates,
    /// Format check only
    Fmt,
    /// Clippy with -D warnings
    Clippy,
    /// cargo deny check
    Deny,
    /// Run tests (cargo-nextest if installed, else cargo test)
    Test,
}

/// Execute a command, returning an error with a `HARNESS VIOLATION` message on failure.
fn run(cmd: &str, args: &[&str]) -> anyhow::Result<()> {
    println!("  → {cmd} {}", args.join(" "));
    let status = Command::new(cmd).args(args).status()?;
    if !status.success() {
        anyhow::bail!("HARNESS VIOLATION: `{cmd} {}` failed", args.join(" "));
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::QualityGates => {
            println!("==> Delegating to scripts/quality-gates.sh (canonical gate)");
            // bash is required for portable script execution across Linux/macOS codespaces.
            run("bash", &["scripts/quality-gates.sh"])?;
        }
        Cmd::Fmt => run("cargo", &["fmt", "--all", "--", "--check"])?,
        Cmd::Clippy => run(
            "cargo",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ],
        )?,
        Cmd::Deny => run("cargo", &["deny", "check"])?,
        Cmd::Test => {
            // Prefer nextest when present; try installing it, then fall back to cargo test.
            let nextest = Command::new("cargo")
                .args(["nextest", "--version"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            if nextest {
                run("cargo", &["nextest", "run", "--workspace"])?;
            } else {
                println!("  → cargo-nextest not found — attempting install...");
                let installed = Command::new("cargo")
                    .args(["install", "cargo-nextest"])
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
                if installed {
                    println!("  ✓ cargo-nextest installed");
                    run("cargo", &["nextest", "run", "--workspace"])?;
                } else {
                    println!("  ⚠ cargo-nextest install failed — falling back to cargo test");
                    run("cargo", &["test", "--workspace"])?;
                }
            }
        }
    }
    Ok(())
}
