//! Quality check execution and status reporting runners.

use crate::commands;
use crate::config::{XtaskConfig, XtaskError};
use crate::quality;
use crate::reporting::{CheckResult, QualityReport};
use crate::telemetry::{
    self, CiTelemetry, TelemetryConfig, TelemetryScope, TelemetryStage, ToolchainInfo,
};
use std::path::Path;

fn get_rfc3339_timestamp() -> String {
    crate::commands::execute_captured("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"]).map_or_else(
        |_| "2026-01-01T00:00:00Z".to_string(),
        |out| out.trim().to_string(),
    )
}

/// Runs the configured quality gate and emits structured telemetry (issue #289).
///
/// # Errors
/// Returns `XtaskError` if command execution fails or quality gate checks fail.
#[expect(
    clippy::too_many_lines,
    reason = "orchestration spans plan/run/report/telemetry; splitting obscures the linear flow"
)]
pub fn handle_quality_run(
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

    // Detailed plan result to inspect scope and fail-closed fallback status
    let plan_result = quality::plan_checks_detailed(config, tier, only, changed_from)?;
    let (scope_mode, scope_packages, scope_fallback) = changed_from.map_or_else(
        || ("all", Vec::new(), false),
        |_base| {
            let cp = plan_result.changed_paths.as_ref();
            let fallback = cp.is_none() || cp.is_some_and(|c| c.fallback_used);
            let affected = cp.map_or_else(Vec::new, |c| {
                crate::changed_paths::affected_crates(&c.changed_files)
            });
            let is_affected = !affected.is_empty();
            (
                if is_affected {
                    "affected-packages"
                } else {
                    "all"
                },
                affected,
                fallback || !is_affected,
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
    let rep_tier = report_tier(config, tier);
    let fingerprint = telemetry::compute_fingerprint(&rep_tier);
    let telemetry = CiTelemetry {
        schema_version: telemetry::SCHEMA_VERSION,
        timestamp: get_rfc3339_timestamp(),
        tier: rep_tier,
        plan_source: "config/xtask.json".to_string(),
        scope: TelemetryScope {
            mode: scope_mode.to_string(),
            packages: scope_packages,
            fallback_used: scope_fallback,
        },
        stages,
        toolchain: ToolchainInfo::capture(),
        fingerprint: Some(fingerprint),
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

/// Explains check selection decisions without running checks.
///
/// # Errors
/// Returns `XtaskError` if tier is invalid or plan computation fails.
pub fn handle_quality_explain(
    config: &XtaskConfig,
    tier: Option<&str>,
    only: Option<&str>,
    changed_from: Option<&str>,
) -> Result<(), XtaskError> {
    let plan = quality::plan_checks_detailed(config, tier, only, changed_from)?;
    let canonical_tier = report_tier(config, tier);
    println!("=== Quality Check Selection Explanation ===");
    println!("Tier: {canonical_tier}");
    if let Some(base) = changed_from {
        println!("Base SHA / Reference: {base}");
        if plan.fallback_used {
            println!(
                "Git state status: UNREADABLE (fail-closed fallback active; selecting all checks)"
            );
        } else if let Some(ref cp) = plan.changed_paths {
            println!("Changed files detected: {}", cp.changed_files.len());
            for f in &cp.changed_files {
                println!("  - {f}");
            }
        }
    } else {
        println!("Base SHA / Reference: none (full workspace run)");
    }
    println!();
    println!(
        "Selected Checks ({} of {}):",
        plan.selected_checks.len(),
        plan.full_tier_checks.len()
    );
    for detail in plan.details.iter().filter(|d| d.selected) {
        println!("  ✓ {:<35} : {}", detail.check.name(), detail.reason);
    }
    println!();
    println!("Skipped Checks:");
    let skipped: Vec<_> = plan.details.iter().filter(|d| !d.selected).collect();
    if skipped.is_empty() {
        println!("  (none)");
    } else {
        for detail in skipped {
            println!("  ⏭ {:<35} : {}", detail.check.name(), detail.reason);
        }
    }
    println!("===========================================");
    Ok(())
}

/// Checks the freshness status of telemetry evidence.
///
/// # Errors
/// Returns `XtaskError::CommandFailure` if evidence is missing, stale, or failed.
pub fn handle_quality_status(config: &XtaskConfig, tier: Option<&str>) -> Result<(), XtaskError> {
    let telemetry_config = TelemetryConfig::load_or_default();
    let artifact_path = Path::new(&telemetry_config.artifact_path);

    if !artifact_path.exists() {
        println!("Evidence Status: MISSING");
        println!(
            "  ! Telemetry artifact '{}' not found.",
            artifact_path.display()
        );
        return Err(XtaskError::CommandFailure {
            command: "quality status".to_string(),
            exit_code: Some(1),
        });
    }

    let content = match std::fs::read_to_string(artifact_path) {
        Ok(c) => c,
        Err(e) => {
            println!("Evidence Status: MISSING");
            println!(
                "  ! Could not read telemetry artifact '{}': {e}",
                artifact_path.display()
            );
            return Err(XtaskError::CommandFailure {
                command: "quality status".to_string(),
                exit_code: Some(1),
            });
        }
    };

    let telemetry: CiTelemetry = match serde_json::from_str(&content) {
        Ok(t) => t,
        Err(e) => {
            println!("Evidence Status: MISSING");
            println!(
                "  ! Telemetry artifact '{}' is invalid JSON: {e}",
                artifact_path.display()
            );
            return Err(XtaskError::CommandFailure {
                command: "quality status".to_string(),
                exit_code: Some(1),
            });
        }
    };

    let target_tier = tier.map_or_else(|| telemetry.tier.as_str(), |t| t);
    let selected_tier = report_tier(config, Some(target_tier));
    let current_fingerprint = telemetry::compute_fingerprint(&selected_tier);
    let status = telemetry.check_freshness(&current_fingerprint);

    println!("Evidence Status: {status}");
    println!("  - Schema Version: {}", telemetry.schema_version);
    println!("  - Tier: {}", telemetry.tier);
    println!("  - Timestamp: {}", telemetry.timestamp);

    match status {
        telemetry::EvidenceStatus::Green => {
            println!("  ✓ Artifact is fresh and all checks passed.");
            Ok(())
        }
        telemetry::EvidenceStatus::Red => {
            println!("  ✗ Last quality run contains failed checks.");
            Err(XtaskError::CommandFailure {
                command: "quality status".to_string(),
                exit_code: Some(1),
            })
        }
        telemetry::EvidenceStatus::Stale => {
            println!("  ! Workspace, policy, or tier state moved on since last run.");
            Err(XtaskError::CommandFailure {
                command: "quality status".to_string(),
                exit_code: Some(1),
            })
        }
        telemetry::EvidenceStatus::Missing => {
            println!("  ! Telemetry evidence missing or invalid.");
            Err(XtaskError::CommandFailure {
                command: "quality status".to_string(),
                exit_code: Some(1),
            })
        }
    }
}

/// Resolves canonical tier name.
#[must_use]
pub fn report_tier(config: &XtaskConfig, tier: Option<&str>) -> String {
    let env = std::env::var(&config.env_var_name).ok();
    let sel = tier.or(env.as_deref()).unwrap_or(&config.default_tier);
    match sel {
        "fast-pr" => "pull-request".to_string(),
        "full-gate" | "all" => "protected-branch".to_string(),
        other => other.to_string(),
    }
}

/// Generates GitHub Actions summary.
///
/// # Errors
/// Returns `XtaskError` if reading report or writing summary fails.
pub fn handle_github_summary() -> Result<(), XtaskError> {
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
