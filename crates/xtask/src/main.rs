//! xtask — thin wrappers around template quality tooling.
pub mod agent_adapters;
pub mod changed_paths;
pub mod commands;
pub mod config;
pub mod path_rules;
pub mod quality;
pub mod quality_helpers;
pub mod quality_runner;
pub mod release;
pub mod reporting;
pub mod telemetry;
pub mod template_init;
pub mod template_profile;
pub mod toolchain;

use clap::{Parser, Subcommand};
use config::{XtaskConfig, XtaskError};
use quality_runner::{handle_github_summary, handle_quality_run, handle_quality_status};

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
    /// Run the full quality gate (equivalent to `scripts/quality-gates.sh`).
    QualityGates {
        /// Autofix what can be fixed before running the gate.
        #[arg(long)]
        fix: bool,
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
    /// Agent adapter validation and inventory.
    Agents {
        #[command(subcommand)]
        sub: AgentsSub,
    },
    /// Release automation.
    Release {
        #[command(subcommand)]
        sub: release::ReleaseSub,
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
        /// Autofix what can be fixed before running the checks.
        #[arg(long)]
        fix: bool,
    },
    /// Check evidence freshness status without re-running checks.
    Status {
        #[arg(long)]
        tier: Option<String>,
    },
}

#[derive(Subcommand)]
enum TemplateSub {
    /// Initialize template from a profile blueprint (see config/template-profiles/).
    Init {
        #[arg(long)]
        profile: String, // e.g. "minimal", "library", "cli", "service", "workspace", "ai-agent"
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
    /// Validate a profile blueprint by id or path.
    ValidateProfile {
        /// Profile id or path to a .toml blueprint.
        #[arg(long)]
        profile: String,
    },
    /// Print an inspection summary of a shipped profile.
    Inspect {
        /// Shipped profile id, e.g. "minimal".
        #[arg(long)]
        profile: String,
    },
}

#[derive(Subcommand)]
enum ReportSub {
    /// Write GHA summary and status markdown files.
    GithubSummary,
}

#[derive(Subcommand)]
enum AgentsSub {
    /// Validate all registered adapters against the canonical contract.
    Validate,
    /// Print an inventory of all registered adapters.
    Inventory {
        /// Output format: "markdown" or "plain".
        #[arg(long, default_value = "markdown")]
        format: String,
    },
    /// Verify that context files (llms.txt, llms-full.txt) exist and are current.
    CheckContext,
}

fn handle_agents_validate() -> Result<(), XtaskError> {
    let manifest = agent_adapters::AgentAdaptersManifest::load()?;
    let result = manifest.validate_from_cwd()?;
    result.print_report();
    if result.is_ok() {
        Ok(())
    } else {
        Err(XtaskError::CommandFailure {
            command: "agents validate".to_string(),
            exit_code: Some(1),
        })
    }
}

fn handle_agents_inventory(format: &str) -> Result<(), XtaskError> {
    let manifest = agent_adapters::AgentAdaptersManifest::load()?;
    match format {
        "markdown" => {
            println!("{}", manifest.inventory_markdown());
            Ok(())
        }
        "plain" => {
            manifest.print_inventory_plain();
            Ok(())
        }
        other => Err(XtaskError::InvalidConfig {
            message: format!("Unknown format '{other}'. Use 'markdown' or 'plain'."),
        }),
    }
}

fn handle_agents_check_context() -> Result<(), XtaskError> {
    agent_adapters::AgentAdaptersManifest::load()?.check_context()
}

fn main() {
    let cli = Cli::parse();
    let config = XtaskConfig::load_from_file("config/xtask.json").unwrap_or_else(|e| {
        eprintln!("Error running xtask: {e}");
        eprintln!(
            "config/xtask.json exists but is invalid; refusing to fall back to defaults (fail-closed)."
        );
        std::process::exit(1);
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
                fix,
            } => handle_quality_run(
                &config,
                tier.as_deref(),
                only.as_deref(),
                changed_from.as_deref(),
                fix,
            ),
            QualitySub::Status { tier } => handle_quality_status(&config, tier.as_deref()),
        },
        Cmd::QualityGates { fix } => handle_quality_run(&config, None, None, None, fix),
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
            TemplateSub::ValidateProfile { profile } => {
                template_profile::TemplateProfile::load_from_path(&profile)
                    .or_else(|_| template_profile::TemplateProfile::load(&profile))
                    .map(|loaded| {
                        println!("  ✓ Profile '{}' is valid", loaded.metadata.id);
                    })
            }
            TemplateSub::Inspect { profile } => {
                template_profile::TemplateProfile::load(&profile).map(|loaded| loaded.inspect())
            }
        },
        Cmd::Report { sub } => match sub {
            ReportSub::GithubSummary => handle_github_summary(),
        },
        Cmd::Agents { sub } => match sub {
            AgentsSub::Validate => handle_agents_validate(),
            AgentsSub::Inventory { format } => handle_agents_inventory(&format),
            AgentsSub::CheckContext => handle_agents_check_context(),
        },
        Cmd::Release { sub } => sub.run(),
    };

    if let Err(e) = result {
        if !matches!(&e, XtaskError::CommandFailure { command, .. } if command == "quality status")
        {
            eprintln!();
            eprintln!("Error running xtask: {e}");
        }
        std::process::exit(1);
    }
}
