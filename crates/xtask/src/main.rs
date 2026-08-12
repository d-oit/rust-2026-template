//! xtask — thin wrappers around template quality tooling.
pub mod agent_adapters;
pub mod changed_paths;
pub mod commands;
pub mod config;
pub mod quality;
pub mod quality_helpers;
pub mod reporting;
pub mod telemetry;
pub mod template_init;
pub mod template_profile;
pub mod toolchain;

use clap::{Parser, Subcommand};
use config::{XtaskConfig, XtaskError};
use reporting::{CheckResult, QualityReport};
use std::path::Path;
use telemetry::{CiTelemetry, TelemetryConfig, TelemetryScope, TelemetryStage, ToolchainInfo};

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

fn get_rfc3339_timestamp() -> String {
    crate::commands::execute_captured("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"]).map_or_else(
        |_| "2026-01-01T00:00:00Z".to_string(),
        |out| out.trim().to_string(),
    )
}

/// Runs the configured quality gate and emits structured telemetry (issue #289).
///
/// Long by design: it orchestrates planning, execution, reporting, and telemetry in one
/// linear sequence so CI can treat the whole gate as a single stage.
#[expect(
    clippy::too_many_lines,
    reason = "orchestration spans plan/run/report/telemetry; splitting obscures the linear flow"
)]
fn handle_quality_run(
    config: &XtaskConfig,
    tier: Option<&str>,
    only: Option<&str>,
    changed_from: Option<&str>,
    fix: bool,
) -> Result<(), XtaskError> {
    if fix {
        println!("Autofix mode: applying cargo fmt and clippy --fix first...");
        commands::execute("cargo", &["fmt", "--all"])?;
        commands::execute(
            "cargo",
            &[
                "clippy",
                "--fix",
                "--allow-dirty",
                "--allow-staged",
                "--all-targets",
                "--all-features",
            ],
        )?;
    }

    // Full tier plan (for skip-reporting) vs the scoped plan actually run.
    let full_checks = quality::plan_checks(config, tier, None, None)?;
    let planned_checks = quality::plan_checks(config, tier, only, changed_from)?;
    println!(
        "Planned checks to execute ({} of {}):",
        planned_checks.len(),
        full_checks.len()
    );
    for check in &planned_checks {
        println!("  - {}", check.name());
    }
    println!();

    let mut results = Vec::new();
    let mut stages: Vec<TelemetryStage> = Vec::new();
    let mut overall_success = true;

    for check in &full_checks {
        let name = check.name().to_string();
        if !planned_checks.contains(check) {
            let reason = if changed_from.is_some() {
                "not affected by changed paths"
            } else {
                "excluded by --only filter"
            };
            stages.push(TelemetryStage {
                id: telemetry::stage_id(&name),
                status: "skipped".to_string(),
                duration_ms: 0,
                cache: "not-applicable".to_string(),
                skipped_reason: Some(reason.to_string()),
            });
            continue;
        }
        let start = std::time::Instant::now();
        let outcome = match quality::run_check(*check, config) {
            Ok(()) => "success",
            Err(e) => {
                overall_success = false;
                results.push(CheckResult {
                    name: name.clone(),
                    status: "failed".to_string(),
                    message: Some(e.to_string()),
                });
                "failed"
            }
        };
        if outcome == "success" {
            results.push(CheckResult {
                name: name.clone(),
                status: "success".to_string(),
                message: Some(format!("Passed in {:?}", start.elapsed())),
            });
        }
        stages.push(TelemetryStage {
            id: telemetry::stage_id(&name),
            status: if outcome == "failed" {
                "failed"
            } else {
                "passed"
            }
            .to_string(),
            duration_ms: u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
            cache: "not-applicable".to_string(),
            skipped_reason: None,
        });
    }

    // Telemetry scope: affected packages when a base was supplied, else the whole workspace.
    let (scope_mode, scope_packages, scope_fallback) = changed_from.map_or_else(
        || ("all", Vec::new(), false),
        |base| {
            let affected = crate::changed_paths::ChangedPaths::from_git(base).map_or_else(
                |_| {
                    println!("  ! Unable to resolve changed paths; falling back to full scope");
                    Vec::new()
                },
                |cp| crate::changed_paths::affected_crates(&cp.changed_files),
            );
            let is_affected = !affected.is_empty();
            (
                if is_affected {
                    "affected-packages"
                } else {
                    "all"
                },
                affected,
                !is_affected,
            )
        },
    );

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

    // Telemetry (issue #289): structured artifact + Markdown summary, always emitted.
    let telemetry_config = TelemetryConfig::load_or_default();
    let telemetry = CiTelemetry {
        schema_version: telemetry::SCHEMA_VERSION,
        timestamp: get_rfc3339_timestamp(),
        tier: report_tier(config, tier),
        plan_source: "config/xtask.json".to_string(),
        scope: TelemetryScope {
            mode: scope_mode.to_string(),
            packages: scope_packages,
            fallback_used: scope_fallback,
        },
        stages,
        toolchain: ToolchainInfo::capture(),
    };
    telemetry.emit(&telemetry_config)?;
    // Surface the telemetry summary in the GHA step summary too, when present.
    if let Ok(summary_path) = std::env::var("GITHUB_STEP_SUMMARY") {
        if !summary_path.is_empty() {
            use std::io::Write as _;
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(&summary_path)
                .map_err(|e| XtaskError::CacheIssue {
                    message: e.to_string(),
                })?;
            writeln!(file, "\n{}", telemetry.summary_markdown(&telemetry_config)).map_err(|e| {
                XtaskError::CacheIssue {
                    message: e.to_string(),
                }
            })?;
        }
    }

    if overall_success {
        Ok(())
    } else {
        Err(XtaskError::CommandFailure {
            command: "quality run".to_string(),
            exit_code: Some(1),
        })
    }
}

fn report_tier(config: &XtaskConfig, tier: Option<&str>) -> String {
    let env = std::env::var(&config.env_var_name).ok();
    let sel = tier.or(env.as_deref()).unwrap_or(&config.default_tier);
    match sel {
        "fast-pr" => "pull-request".to_string(),
        "full-gate" | "all" => "protected-branch".to_string(),
        other => other.to_string(),
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

/// Resolves the repository root from the manifest path for CWD-independent validation.
fn repo_root_from_manifest() -> Result<std::path::PathBuf, XtaskError> {
    let manifest_path = std::path::Path::new(agent_adapters::MANIFEST_PATH);
    // The manifest lives at .agents/agent-adapters.toml — its grandparent is the repo root.
    Ok(if manifest_path.exists() {
        manifest_path
            .canonicalize()
            .map_err(|e| XtaskError::InvalidConfig {
                message: format!("Failed to resolve manifest path: {e}"),
            })?
            .parent()
            .and_then(|p| p.parent())
            .map_or_else(
                || std::path::PathBuf::from("."),
                std::path::Path::to_path_buf,
            )
    } else {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    })
}

fn handle_agents_validate() -> Result<(), XtaskError> {
    let manifest = agent_adapters::AgentAdaptersManifest::load()?;
    let repo_root = repo_root_from_manifest()?;
    let result = manifest.validate(&repo_root)?;
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
        }
        "plain" => {
            println!("Adapters:");
            for adapter in &manifest.adapters {
                println!(
                    "  {} → {} ({})",
                    adapter.id, adapter.entrypoint, adapter.role
                );
            }
        }
        other => {
            return Err(XtaskError::InvalidConfig {
                message: format!("Unknown format '{other}'. Use 'markdown' or 'plain'."),
            });
        }
    }
    Ok(())
}

fn handle_agents_check_context() -> Result<(), XtaskError> {
    let manifest = agent_adapters::AgentAdaptersManifest::load()?;
    let mut ok = true;
    for ctx_file in &manifest.contract.context_files {
        if Path::new(ctx_file).exists() {
            println!("  ✅ {ctx_file}");
        } else {
            println!("  ❌ {ctx_file} — missing");
            ok = false;
        }
    }
    if ok {
        Ok(())
    } else {
        Err(XtaskError::CommandFailure {
            command: "agents check-context".to_string(),
            exit_code: Some(1),
        })
    }
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
                fix,
            } => handle_quality_run(
                &config,
                tier.as_deref(),
                only.as_deref(),
                changed_from.as_deref(),
                fix,
            ),
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
    };

    if let Err(e) = result {
        eprintln!();
        eprintln!("Error running xtask: {e}");
        std::process::exit(1);
    }
}
