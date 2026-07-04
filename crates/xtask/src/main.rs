//! xtask — Quality gate orchestration for rust-2026-template.
//!
//! Run quality checks with:
//! ```bash
//! cargo run -p xtask quality-gates   # Run all quality gates
//! cargo run -p xtask fmt             # Format check only
//! cargo run -p xtask clippy          # Clippy with -D warnings
//! cargo run -p xtask deny            # cargo deny check
//! cargo run -p xtask test            # Run tests
//! ```
#![forbid(unsafe_code)]
#![allow(clippy::print_stdout, clippy::print_stderr)]

use clap::{Parser, Subcommand};
use std::process::Command;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Quality gate orchestration for rust-2026-template")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run all quality gates (fmt, clippy, deny, test)
    QualityGates,
    /// Format check only
    Fmt,
    /// Clippy with -D warnings
    Clippy,
    /// cargo deny check
    Deny,
    /// Run tests
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
            println!("==> Running all quality gates");
            run("cargo", &["fmt", "--all", "--", "--check"])?;
            run(
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
            )?;
            run("cargo", &["deny", "check"])?;
            run("cargo", &["nextest", "run", "--workspace"])?;
            println!("✅ All quality gates passed");
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
        Cmd::Test => run("cargo", &["nextest", "run", "--workspace"])?,
    }
    Ok(())
}
