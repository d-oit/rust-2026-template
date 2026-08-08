//! xtask — thin wrappers around template quality tooling.
pub mod changed_paths;
pub mod commands;
pub mod config;
pub mod quality;
pub mod quality_helpers;
pub mod reporting;
pub mod template_init;
pub mod toolchain;

use clap::{Parser, Subcommand};
use config::{XtaskConfig, XtaskError};
use reporting::{CheckResult, QualityReport};
use std::path::Path;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Xtask automation runner")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run diagnostics on the current environment.
    Doctor,
    /// Quality gate controls.
    Quality {
        #[command(subcommand)]
        sub: QualitySub,
    },
    /// Template initialization.
    Template {
        #[command(subcommand)]
        sub: TemplateSub,
    },
    /// Generate GitHub Actions summary.
    Report {
        #[command(subcommand)]
        sub: ReportSub,
    },
}

#[derive(Subcommand)]
enum QualitySub {
    /// Plan quality checks.
    Plan {
        #[arg(long)]
        tier: Option<String>,
        #[arg(long)]
        only: Option<String>,
        #[arg(long)]
        changed_from: Option<String>,
    },
    /// Run quality checks.
    Run {
        #[arg(long)]
        tier: Option<String>,
        #[arg(long)]
        only: Option<String>,
        #[arg(long)]
        changed_from: Option<String>,
    },
}

#[derive(Subcommand)]
enum TemplateSub {
    /// Initialize template from profile.
    Init {
        #[arg(long)]
        profile: String, // "minimal" or "full"
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        author: Option<String>,
        #[arg(long)]
        repo: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum ReportSub {
    /// Write GHA summary and status markdown files.
    GithubSummary,
}

fn get_rfc3339_timestamp() -> String {
    crate::commands::execute_captured("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"]).map_or_else(
        |_| "2026-01-01T00:00:00Z".to_string(),
        |out| out.trim().to_string(),
    )
}

fn handle_quality_run(
    config: &XtaskConfig,
    tier: Option<&str>,
    only: Option<&str>,
    changed_from: Option<&str>,
) -> Result<(), XtaskError> {
    let planned_checks = quality::plan_checks(config, tier, only, changed_from)?;
    println!("Planned checks to execute:");
    for check in &planned_checks {
        println!("  - {}", check.name());
    }
    println!();

    let mut results = Vec::new();
    let mut overall_success = true;

    for check in planned_checks {
        let name = check.name().to_string();
        let start = std::time::Instant::now();
        match quality::run_check(check, config) {
            Ok(()) => {
                results.push(CheckResult {
                    name,
                    status: "success".to_string(),
                    message: Some(format!("Passed in {:?}", start.elapsed())),
                });
            }
            Err(e) => {
                overall_success = false;
                results.push(CheckResult {
                    name,
                    status: "failed".to_string(),
                    message: Some(e.to_string()),
                });
            }
        }
    }

    let commit_sha = std::env::var("GITHUB_SHA").unwrap_or_else(|_| {
        commands::execute_captured("git", &["rev-parse", "HEAD"])
            .unwrap_or_else(|_| "unknown_sha".to_string())
            .trim()
            .to_string()
    });

    let branch_name = std::env::var("GITHUB_REF_NAME").unwrap_or_else(|_| {
        commands::execute_captured("git", &["branch", "--show-current"])
            .unwrap_or_else(|_| "unknown_branch".to_string())
            .trim()
            .to_string()
    });

    let report = QualityReport {
        timestamp: get_rfc3339_timestamp(),
        commit: commit_sha,
        branch: branch_name,
        checks: results,
        overall: if overall_success {
            "success".to_string()
        } else {
            "failure".to_string()
        },
    };

    report.print_console();
    report.write_json_report()?;
    report.write_github_summary()?;

    if overall_success {
        Ok(())
    } else {
        Err(XtaskError::CommandFailure {
            command: "quality run".to_string(),
            exit_code: Some(1),
        })
    }
}

fn handle_github_summary() -> Result<(), XtaskError> {
    // Read reports/quality-report.json or .agents/ci/ci-status.json
    let report_path = Path::new(".agents/ci/ci-status.json");
    if report_path.exists() {
        let file_content =
            std::fs::read_to_string(report_path).map_err(|e| XtaskError::CacheIssue {
                message: e.to_string(),
            })?;
        let report: QualityReport =
            serde_json::from_str(&file_content).map_err(|e| XtaskError::InvalidConfig {
                message: e.to_string(),
            })?;
        report.write_github_summary()?;
        println!("  ✓ GitHub Actions summary generated from ci-status.json");
    } else {
        println!("  ! Warning: .agents/ci/ci-status.json not found. No summary to generate.");
    }
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    let config = XtaskConfig::load_from_file("config/xtask.json").unwrap_or_else(|e| {
        println!("  ! Failed to load config/xtask.json, using defaults. Error: {e}");
        XtaskConfig::default()
    });

    let result = match cli.cmd {
        Cmd::Doctor => toolchain::run_doctor(),
        Cmd::Quality { sub } => match sub {
            QualitySub::Plan {
                tier,
                only,
                changed_from,
            } => {
                match quality::plan_checks(
                    &config,
                    tier.as_deref(),
                    only.as_deref(),
                    changed_from.as_deref(),
                ) {
                    Ok(checks) => {
                        println!("Planned Checks:");
                        for check in checks {
                            println!("  - {}", check.name());
                        }
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            }
            QualitySub::Run {
                tier,
                only,
                changed_from,
            } => handle_quality_run(
                &config,
                tier.as_deref(),
                only.as_deref(),
                changed_from.as_deref(),
            ),
        },
        Cmd::Template { sub } => match sub {
            TemplateSub::Init {
                profile,
                name,
                description,
                author,
                repo,
                dry_run,
            } => template_init::run_init(
                &profile,
                name.as_deref(),
                description.as_deref(),
                author.as_deref(),
                repo.as_deref(),
                dry_run,
            ),
        },
        Cmd::Report { sub } => match sub {
            ReportSub::GithubSummary => handle_github_summary(),
        },
    };

    if let Err(e) = result {
        eprintln!();
        eprintln!("Error running xtask: {e}");
        std::process::exit(1);
    }
}
