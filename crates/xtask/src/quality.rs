//! Quality gate planning and execution.
#![allow(clippy::unwrap_used)]

use crate::changed_paths::ChangedPaths;
use crate::commands;
use crate::config::{XtaskConfig, XtaskError};
use crate::quality_helpers;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[path = "quality_policy.rs"]
mod quality_policy;

/// Enum representing the individual quality gate checks.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, serde::Serialize, serde::Deserialize,
)]
pub enum QualityCheck {
    /// Line count limit validation.
    LocLimits,
    /// Skill validation scripts.
    SkillValidation,
    /// ADR compliance check.
    AdrCompliance,
    /// rustfmt checks.
    Fmt,
    /// clippy lints.
    Clippy,
    /// cargo build verify.
    Build,
    /// cargo tests.
    Test,
    /// rust documentation tests.
    DocTest,
    /// cargo audit security checks.
    Audit,
    /// cargo deny check.
    Deny,
    /// cargo machete unused deps.
    Machete,
    /// MSRV compliance audit.
    Msrv,
    /// shellcheck scripts verify.
    ShellCheck,
    /// markdownlint cli formatting.
    MarkdownLint,
    /// scan for accidental email leaks.
    PrivacyCheck,
    /// scan for accidental secret/token leaks.
    SecretScan,
    /// GitHub actions workflows verification.
    WorkflowValidation,
    /// skill evaluations verification.
    SkillEvals,
    /// LLM context files check.
    LlmContext,
    /// CI status json schema and presence.
    CiStatusArtifact,
    /// roast scorer execution.
    RoastScorer,
}

impl QualityCheck {
    /// Human-readable name of the quality check.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::LocLimits => "LOC Limits",
            Self::SkillValidation => "Skill Validation",
            Self::AdrCompliance => "ADR Compliance",
            Self::Fmt => "Rust Format",
            Self::Clippy => "Rust Clippy",
            Self::Build => "Rust Build",
            Self::Test => "Rust Tests",
            Self::DocTest => "Rust Doc Tests",
            Self::Audit => "Rust Security Audit",
            Self::Deny => "Rust Dependency Policy (Deny)",
            Self::Machete => "Rust Unused Dependencies (Machete)",
            Self::Msrv => "Rust MSRV Audit",
            Self::ShellCheck => "Shell Script Lint (ShellCheck)",
            Self::MarkdownLint => "Markdown Lint (markdownlint-cli2)",
            Self::PrivacyCheck => "Privacy Check (No emails)",
            Self::SecretScan => "Secret Scan",
            Self::WorkflowValidation => "GitHub Actions Workflow Validation",
            Self::SkillEvals => "Skill Evaluations",
            Self::LlmContext => "LLM Context Files Check",
            Self::CiStatusArtifact => "CI Status Artifact Check",
            Self::RoastScorer => "Roast Scorer",
        }
    }
}

/// Individual check selection status and explanation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DetailedCheckSelection {
    /// The quality check.
    pub check: QualityCheck,
    /// Whether the check was selected to run.
    pub selected: bool,
    /// Explanation reason for selection or skipping.
    pub reason: String,
}

/// Consolidated plan checks result.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PlanChecksResult {
    /// Full set of checks defined for the active tier.
    pub full_tier_checks: Vec<QualityCheck>,
    /// List of checks selected to run.
    pub selected_checks: Vec<QualityCheck>,
    /// Detailed selection decisions for every check in the tier.
    pub details: Vec<DetailedCheckSelection>,
    /// True if git state could not be resolved and fail-closed fallback was used.
    pub fallback_used: bool,
    /// Changed paths information if `--changed-from` was supplied.
    pub changed_paths: Option<ChangedPaths>,
}

fn matches_only_filter(check: QualityCheck, o_list: &[&str]) -> bool {
    let ch_name = check.name().to_lowercase();
    o_list.iter().any(|&o| {
        let o_lower = o.to_lowercase();
        ch_name.contains(&o_lower)
            || match check {
                QualityCheck::LocLimits => o_lower == "loc",
                QualityCheck::Fmt => o_lower == "fmt" || o_lower == "format",
                QualityCheck::Clippy => o_lower == "clippy" || o_lower == "lint",
                QualityCheck::Build => o_lower == "build",
                QualityCheck::Test => o_lower == "test" || o_lower == "tests",
                QualityCheck::Audit => o_lower == "audit",
                QualityCheck::Deny => o_lower == "deny",
                QualityCheck::Machete => o_lower == "machete" || o_lower == "deps",
                QualityCheck::ShellCheck => o_lower == "shell" || o_lower == "shellcheck",
                QualityCheck::MarkdownLint => o_lower == "markdown" || o_lower == "md",
                QualityCheck::PrivacyCheck => o_lower == "privacy",
                QualityCheck::SecretScan => o_lower == "secret",
                _ => false,
            }
    })
}

fn check_matches_when_changed(
    check: QualityCheck,
    config: &XtaskConfig,
    cp: &ChangedPaths,
) -> (bool, String) {
    if cp.fallback_used {
        return (
            true,
            "selected (unreadable git state; fail-closed fallback active)".to_string(),
        );
    }
    let Some(patterns) = config.when_changed.get(&check) else {
        return (
            true,
            "always run (no when_changed rule for check)".to_string(),
        );
    };
    if patterns.is_empty() {
        return (
            true,
            "always run (no when_changed pattern specified)".to_string(),
        );
    }
    for pattern in patterns {
        if cp
            .changed_files
            .iter()
            .any(|file| crate::config::glob_match(pattern, file))
        {
            return (true, format!("matched glob pattern '{pattern}'"));
        }
    }
    (
        false,
        format!(
            "no changed files matched patterns [{}]",
            patterns.join(", ")
        ),
    )
}

/// Determine which check variants to run based on tier, `--only`, and `--changed-from` with detailed explanations.
///
/// # Errors
/// Returns `XtaskError` if an invalid tier is specified.
pub fn plan_checks_detailed(
    config: &XtaskConfig,
    tier: Option<&str>,
    only: Option<&str>,
    changed_from: Option<&str>,
) -> Result<PlanChecksResult, XtaskError> {
    let env_tier = std::env::var(&config.env_var_name).ok();
    let selected_tier = tier.or(env_tier.as_deref()).unwrap_or(&config.default_tier);
    let canonical_tier = crate::config::canonical_tier_name(selected_tier);
    let Some(def) = config.tiers.get(canonical_tier) else {
        return Err(XtaskError::InvalidConfig {
            message: format!("Unsupported or unconfigured quality tier: {selected_tier}"),
        });
    };
    quality_policy::set_active_tier(canonical_tier);
    let full_tier_checks = def.checks.clone();
    let only_checks: Option<Vec<&str>> =
        only.map(|only_str| only_str.split(',').map(str::trim).collect());

    let cp_opt = match changed_from {
        Some(base_sha) => Some(ChangedPaths::from_git(base_sha)?),
        None => None,
    };
    let fallback_used = cp_opt.as_ref().is_some_and(|cp| cp.fallback_used);

    let mut details = Vec::new();
    let mut selected_checks = Vec::new();

    for check in &full_tier_checks {
        if let Some(ref o_list) = only_checks {
            if !matches_only_filter(*check, o_list) {
                details.push(DetailedCheckSelection {
                    check: *check,
                    selected: false,
                    reason: "excluded by --only filter".to_string(),
                });
                continue;
            }
        }

        let (selected, reason) = cp_opt.as_ref().map_or_else(
            || (true, "selected (full tier run)".to_string()),
            |cp| check_matches_when_changed(*check, config, cp),
        );

        if selected {
            selected_checks.push(*check);
        }
        details.push(DetailedCheckSelection {
            check: *check,
            selected,
            reason,
        });
    }

    Ok(PlanChecksResult {
        full_tier_checks,
        selected_checks,
        details,
        fallback_used,
        changed_paths: cp_opt,
    })
}

/// Determine which check variants to run based on tier, `--only`, and `--changed-from`.
///
/// # Errors
/// Returns `XtaskError` if an invalid tier is specified.
pub fn plan_checks(
    config: &XtaskConfig,
    tier: Option<&str>,
    only: Option<&str>,
    changed_from: Option<&str>,
) -> Result<Vec<QualityCheck>, XtaskError> {
    let result = plan_checks_detailed(config, tier, only, changed_from)?;
    Ok(result.selected_checks)
}

/// Executes a single quality check.
///
/// # Errors
/// Returns `XtaskError` if the check execution reports a failure or error.
pub fn run_check(check: QualityCheck, config: &XtaskConfig) -> Result<(), XtaskError> {
    println!("--- Running Check: {} ---", check.name());
    match check {
        QualityCheck::LocLimits => run_loc_limits_check(config)?,
        QualityCheck::SkillValidation => run_skill_validation()?,
        QualityCheck::AdrCompliance => run_adr_compliance()?,
        QualityCheck::Fmt => run_fmt_check()?,
        QualityCheck::Clippy => run_clippy_check(config)?,
        QualityCheck::Build => run_build_check()?,
        QualityCheck::Test => run_test_check()?,
        QualityCheck::DocTest => run_doc_test()?,
        QualityCheck::Audit => run_audit_check(config)?,
        QualityCheck::Deny => run_deny_check(config)?,
        QualityCheck::Machete => run_machete_check(config)?,
        QualityCheck::Msrv => run_msrv_check(config)?,
        QualityCheck::ShellCheck => run_shellcheck_check(config)?,
        QualityCheck::MarkdownLint => run_markdownlint_check(config)?,
        QualityCheck::PrivacyCheck => quality_helpers::run_privacy_check()?,
        QualityCheck::SecretScan => quality_helpers::run_secret_scan()?,
        QualityCheck::WorkflowValidation => run_workflow_validation(config)?,
        QualityCheck::SkillEvals => run_skill_evals()?,
        QualityCheck::LlmContext => run_llm_context_check()?,
        QualityCheck::CiStatusArtifact => run_ci_status_check()?,
        QualityCheck::RoastScorer => run_roast_scorer()?,
    }
    Ok(())
}

fn run_loc_limits_check(config: &XtaskConfig) -> Result<(), XtaskError> {
    let max_lines = config.lint_thresholds.max_lines_per_file;
    let mut rs_files = Vec::new();
    quality_helpers::find_files(Path::new("."), "rs", &mut rs_files);
    let mut violations = 0;
    for file in rs_files {
        let lines = quality_helpers::count_lines(&file)?;
        if lines > max_lines {
            println!("  ! {}: {lines} lines (max {max_lines})", file.display());
            violations += 1;
        }
    }
    if violations > 0 {
        return Err(XtaskError::InvalidConfig {
            message: format!("LOC: {violations} file(s) exceed maximum of {max_lines} lines"),
        });
    }
    println!("  ✓ All source files within line limits");
    Ok(())
}

fn run_skill_validation() -> Result<(), XtaskError> {
    if Path::new("scripts/validate-skills.sh").exists() {
        commands::execute("bash", &["scripts/validate-skills.sh"])?;
    } else {
        println!("  ! scripts/validate-skills.sh not found, skipping");
    }
    Ok(())
}

fn run_adr_compliance() -> Result<(), XtaskError> {
    if Path::new("scripts/check-adr-compliance.sh").exists() {
        commands::execute("bash", &["scripts/check-adr-compliance.sh"])?;
    } else {
        println!("  ! scripts/check-adr-compliance.sh not found, skipping");
    }
    Ok(())
}

fn run_fmt_check() -> Result<(), XtaskError> {
    commands::execute("cargo", &["fmt", "--all", "--", "--check"])
}

fn run_clippy_check(config: &XtaskConfig) -> Result<(), XtaskError> {
    let mut clippy_args = vec!["clippy", "--workspace", "--all-targets", "--all-features"];
    if config.lint_thresholds.clippy_warnings_as_errors {
        clippy_args.extend(&["--", "-D", "warnings"]);
    }
    commands::execute("cargo", &clippy_args)
}

fn run_build_check() -> Result<(), XtaskError> {
    commands::execute("cargo", &["build", "--workspace", "--all-targets"])
}

fn run_test_check() -> Result<(), XtaskError> {
    let has_nextest = commands::execute_captured("cargo", &["nextest", "--version"]).is_ok();
    if has_nextest {
        commands::execute(
            "cargo",
            &["nextest", "run", "--all-features", "--workspace"],
        )
    } else {
        commands::execute("cargo", &["test", "--all-features", "--workspace"])
    }
}

fn run_doc_test() -> Result<(), XtaskError> {
    commands::execute("cargo", &["test", "--doc", "--all-features"])
}

fn run_audit_check(config: &XtaskConfig) -> Result<(), XtaskError> {
    quality_policy::run_with_tool_policy(config, QualityCheck::Audit, || {
        commands::execute("cargo", &["audit"])
    })
}

fn run_deny_check(config: &XtaskConfig) -> Result<(), XtaskError> {
    quality_policy::run_with_tool_policy(config, QualityCheck::Deny, || {
        commands::execute("cargo", &["deny", "check"])
    })
}

fn run_machete_check(config: &XtaskConfig) -> Result<(), XtaskError> {
    quality_policy::run_with_tool_policy(config, QualityCheck::Machete, || {
        commands::execute("cargo-machete", &[])
    })
}

fn run_msrv_check(config: &XtaskConfig) -> Result<(), XtaskError> {
    quality_policy::run_with_tool_policy(config, QualityCheck::Msrv, || {
        commands::execute("bash", &["scripts/audit-msrv.sh"])
    })
}

fn run_shellcheck_check(config: &XtaskConfig) -> Result<(), XtaskError> {
    quality_policy::run_with_tool_policy(config, QualityCheck::ShellCheck, || {
        let mut sh_files = Vec::new();
        quality_helpers::find_files(Path::new("."), "sh", &mut sh_files);
        if sh_files.is_empty() {
            println!("  ✓ No shell scripts detected");
            return Ok(());
        }
        let mut args = vec!["--severity=error"];
        let sh_strs: Vec<String> = sh_files
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        for s in &sh_strs {
            args.push(s);
        }
        commands::execute("shellcheck", &args)
    })
}

fn run_markdownlint_check(config: &XtaskConfig) -> Result<(), XtaskError> {
    quality_policy::run_with_tool_policy(config, QualityCheck::MarkdownLint, || {
        commands::execute("markdownlint-cli2", &["**/*.md"])
    })
}

fn run_workflow_validation(config: &XtaskConfig) -> Result<(), XtaskError> {
    quality_policy::run_with_tool_policy(config, QualityCheck::WorkflowValidation, || {
        commands::execute("bash", &["scripts/validate-workflows.sh"])
    })
}

fn run_skill_evals() -> Result<(), XtaskError> {
    if Path::new("scripts/run-evals.sh").exists() {
        commands::execute("bash", &["scripts/run-evals.sh"])
    } else {
        println!("  ! scripts/run-evals.sh not found, skipping");
        Ok(())
    }
}

fn run_llm_context_check() -> Result<(), XtaskError> {
    let files = &["llms.txt", "llms-full.txt"];
    for f in files {
        if !Path::new(f).exists() {
            return Err(XtaskError::InvalidConfig {
                message: format!(
                    "LLM context file '{f}' missing. Run scripts/generate-llms-txt.sh"
                ),
            });
        }
    }
    println!("  ✓ llms.txt and llms-full.txt are present");
    Ok(())
}

fn run_ci_status_check() -> Result<(), XtaskError> {
    let path = Path::new(".agents/ci/ci-status.json");
    if path.exists() {
        let mut file = File::open(path).map_err(|e| XtaskError::InvalidConfig {
            message: e.to_string(),
        })?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| XtaskError::InvalidConfig {
                message: e.to_string(),
            })?;
        let _v: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| XtaskError::InvalidConfig {
                message: format!("CI status artifact is invalid JSON: {e}"),
            })?;
        println!("  ✓ CI status artifact exists and is valid JSON");
    } else {
        println!("  ! CI status artifact .agents/ci/ci-status.json not found, skipping check");
    }
    Ok(())
}

fn run_roast_scorer() -> Result<(), XtaskError> {
    if Path::new("scripts/roast-scorer.sh").exists() {
        commands::execute("bash", &["scripts/roast-scorer.sh"])
    } else {
        println!("  ! scripts/roast-scorer.sh not found, skipping");
        Ok(())
    }
}

#[cfg(test)]
#[path = "quality_test.rs"]
mod tests;
