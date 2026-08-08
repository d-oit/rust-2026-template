//! Quality gate planning and execution.
#![allow(clippy::too_many_lines, clippy::unwrap_used)]

use crate::changed_paths::ChangedPaths;
use crate::commands;
use crate::config::{XtaskConfig, XtaskError};
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Enum representing the individual quality gate checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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

/// Helper to recursively find files with a given extension.
fn find_files(dir: &Path, ext: &str, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name != "target" && name != ".git" && name != ".cargo" && name != "node_modules" {
                    find_files(&path, ext, files);
                }
            } else if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some(ext) {
                files.push(path);
            }
        }
    }
}

/// Helper to recursively find all files.
fn find_files_all(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name != "target" && name != ".git" && name != ".cargo" && name != "node_modules" {
                    find_files_all(&path, files);
                }
            } else if path.is_file() {
                files.push(path);
            }
        }
    }
}

/// Helper to count the number of lines in a file.
fn count_lines(path: &Path) -> Result<usize, XtaskError> {
    let file = File::open(path).map_err(|e| XtaskError::InvalidConfig {
        message: format!("Failed to open file for line counting: {e}"),
    })?;
    let reader = BufReader::new(file);
    let mut count = 0;
    for _ in reader.lines() {
        count += 1;
    }
    Ok(count)
}

/// Determine which check variants to run based on tier, `--only`, and `--changed-from`.
///
/// # Errors
/// Returns `XtaskError` if git diff command fails or if an invalid tier is specified.
pub fn plan_checks(
    config: &XtaskConfig,
    tier: Option<&str>,
    only: Option<&str>,
    changed_from: Option<&str>,
) -> Result<Vec<QualityCheck>, XtaskError> {
    let selected_tier = tier.unwrap_or(&config.default_tier);
    let mut checks = match selected_tier {
        "fast-pr" => vec![
            QualityCheck::LocLimits,
            QualityCheck::Fmt,
            QualityCheck::Clippy,
            QualityCheck::Build,
            QualityCheck::Test,
            QualityCheck::DocTest,
            QualityCheck::PrivacyCheck,
            QualityCheck::SecretScan,
        ],
        "full-gate" | "all" => vec![
            QualityCheck::LocLimits,
            QualityCheck::SkillValidation,
            QualityCheck::AdrCompliance,
            QualityCheck::Fmt,
            QualityCheck::Clippy,
            QualityCheck::Build,
            QualityCheck::Test,
            QualityCheck::DocTest,
            QualityCheck::Audit,
            QualityCheck::Deny,
            QualityCheck::Machete,
            QualityCheck::Msrv,
            QualityCheck::ShellCheck,
            QualityCheck::MarkdownLint,
            QualityCheck::PrivacyCheck,
            QualityCheck::SecretScan,
            QualityCheck::WorkflowValidation,
            QualityCheck::SkillEvals,
            QualityCheck::LlmContext,
            QualityCheck::CiStatusArtifact,
            QualityCheck::RoastScorer,
        ],
        _ => {
            return Err(XtaskError::InvalidConfig {
                message: format!("Unsupported quality tier: {selected_tier}"),
            });
        }
    };

    if let Some(only_str) = only {
        let only_checks: Vec<&str> = only_str.split(',').map(str::trim).collect();
        checks.retain(|check| {
            let ch_name = check.name().to_lowercase();
            only_checks.iter().any(|&o| {
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
        });
    }

    if let Some(base_sha) = changed_from {
        let cp = ChangedPaths::from_git(base_sha)?;
        checks.retain(|check| match check {
            QualityCheck::Fmt
            | QualityCheck::Clippy
            | QualityCheck::Build
            | QualityCheck::Test
            | QualityCheck::DocTest
            | QualityCheck::Audit
            | QualityCheck::Deny
            | QualityCheck::Machete => cp.has_code_changes,

            QualityCheck::ShellCheck => cp.has_shell_changes,
            QualityCheck::MarkdownLint => cp.has_markdown_changes,
            _ => true,
        });
    }

    Ok(checks)
}

fn run_privacy_check() -> Result<(), XtaskError> {
    let email_re = Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}")
        .map_err(|e| XtaskError::InvalidConfig { message: e.to_string() })?;

    let exclude_re = Regex::new(r"example\.com|example\.org|test\.com|\.git|target|\.opencode|\.mimocode|\.cargo|node_modules")
        .map_err(|e| XtaskError::InvalidConfig { message: e.to_string() })?;

    let mut files = Vec::new();
    let is_git = Command::new("git").arg("rev-parse").arg("--git-dir")
        .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status()
        .map(|s| s.success()).unwrap_or(false);

    if is_git {
        if let Ok(out) = commands::execute_captured("git", &["ls-files"]) {
            files = out.lines().map(|s| PathBuf::from(s.trim())).collect();
        }
    }

    if files.is_empty() {
        find_files_all(Path::new("."), &mut files);
    }

    let mut violations = 0;
    for file_path in files {
        if !file_path.is_file() {
            continue;
        }
        let file_str = file_path.to_string_lossy();
        if exclude_re.is_match(&file_str) {
            continue;
        }

        let file = File::open(&file_path).map_err(|e| XtaskError::InvalidConfig { message: e.to_string() })?;
        let mut handle = file.take(1_048_576);
        let mut content = String::new();
        if handle.read_to_string(&mut content).is_ok() {
            for line in content.lines() {
                if email_re.is_match(line) && !exclude_re.is_match(line) {
                    println!("  ! Email detected in {}: {}", file_path.display(), line.trim());
                    violations += 1;
                }
            }
        }
    }

    if violations > 0 {
        return Err(XtaskError::InvalidConfig {
            message: format!("Privacy: {violations} email address(es) detected in codebase"),
        });
    }
    println!("  ✓ Privacy Check OK (No non-test emails found)");
    Ok(())
}

fn run_secret_scan() -> Result<(), XtaskError> {
    let secret_re = Regex::new(r#"(api_key|token|secret|password|auth|key)[[:space:]]*[:=][[:space:]]*['"][a-zA-Z0-9_-]{16,}['"]"#)
        .map_err(|e| XtaskError::InvalidConfig { message: e.to_string() })?;

    let exclude_re = Regex::new(r"example\.com|example\.org|test\.com|GITHUB_TOKEN|CARGO_REGISTRY_TOKEN|worktree|\.git|target|\.cargo|\.agents|\.opencode|node_modules")
        .map_err(|e| XtaskError::InvalidConfig { message: e.to_string() })?;

    let mut files = Vec::new();
    let is_git = Command::new("git").arg("rev-parse").arg("--git-dir")
        .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status()
        .map(|s| s.success()).unwrap_or(false);

    if is_git {
        if let Ok(out) = commands::execute_captured("git", &["ls-files"]) {
            files = out.lines().map(|s| PathBuf::from(s.trim())).collect();
        }
    }

    if files.is_empty() {
        find_files_all(Path::new("."), &mut files);
    }

    let mut violations = 0;
    for file_path in files {
        if !file_path.is_file() {
            continue;
        }
        let file_str = file_path.to_string_lossy();
        if exclude_re.is_match(&file_str) {
            continue;
        }

        let file = File::open(&file_path).map_err(|e| XtaskError::InvalidConfig { message: e.to_string() })?;
        let mut handle = file.take(1_048_576);
        let mut content = String::new();
        if handle.read_to_string(&mut content).is_ok() {
            for line in content.lines() {
                if secret_re.is_match(line) && !exclude_re.is_match(line) {
                    println!("  ! Secret pattern detected in {}: {}", file_path.display(), line.trim());
                    violations += 1;
                }
            }
        }
    }

    if violations > 0 {
        return Err(XtaskError::InvalidConfig {
            message: format!("Secret Scan: {violations} potential secret(s) detected in codebase"),
        });
    }
    println!("  ✓ Secret Scan OK (No potential secrets found)");
    Ok(())
}

/// Executes a single quality check.
///
/// # Errors
/// Returns `XtaskError` if the check execution reports a failure or error.
pub fn run_check(check: QualityCheck, config: &XtaskConfig) -> Result<(), XtaskError> {
    println!("--- Running Check: {} ---", check.name());
    match check {
        QualityCheck::LocLimits => {
            let max_lines = config.lint_thresholds.max_lines_per_file;
            let mut rs_files = Vec::new();
            find_files(Path::new("."), "rs", &mut rs_files);
            let mut violations = 0;
            for file in rs_files {
                let lines = count_lines(&file)?;
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
        }
        QualityCheck::SkillValidation => {
            if Path::new("scripts/validate-skills.sh").exists() {
                commands::execute("bash", &["scripts/validate-skills.sh"])?;
            } else {
                println!("  ! scripts/validate-skills.sh not found, skipping");
            }
        }
        QualityCheck::AdrCompliance => {
            if Path::new("scripts/check-adr-compliance.sh").exists() {
                commands::execute("bash", &["scripts/check-adr-compliance.sh"])?;
            } else {
                println!("  ! scripts/check-adr-compliance.sh not found, skipping");
            }
        }
        QualityCheck::Fmt => {
            commands::execute("cargo", &["fmt", "--all", "--", "--check"])?;
        }
        QualityCheck::Clippy => {
            let mut clippy_args = vec!["clippy", "--workspace", "--all-targets", "--all-features"];
            if config.lint_thresholds.clippy_warnings_as_errors {
                clippy_args.extend(&["--", "-D", "warnings"]);
            }
            commands::execute("cargo", &clippy_args)?;
        }
        QualityCheck::Build => {
            commands::execute("cargo", &["build", "--workspace", "--all-targets"])?;
        }
        QualityCheck::Test => {
            let has_nextest = commands::execute_captured("cargo", &["nextest", "--version"]).is_ok();
            if has_nextest {
                commands::execute("cargo", &["nextest", "run", "--all-features", "--workspace"])?;
            } else {
                commands::execute("cargo", &["test", "--all-features", "--workspace"])?;
            }
        }
        QualityCheck::DocTest => {
            commands::execute("cargo", &["test", "--doc", "--all-features"])?;
        }
        QualityCheck::Audit => {
            let has_audit = commands::execute_captured("cargo", &["audit", "--version"]).is_ok();
            if has_audit {
                commands::execute("cargo", &["audit"])?;
            } else {
                println!("  ! cargo-audit not found, skipping");
            }
        }
        QualityCheck::Deny => {
            let has_deny = commands::execute_captured("cargo", &["deny", "--version"]).is_ok();
            if has_deny {
                commands::execute("cargo", &["deny", "check"])?;
            } else {
                println!("  ! cargo-deny not found, skipping");
            }
        }
        QualityCheck::Machete => {
            let has_machete = commands::execute_captured("cargo-machete", &["--version"]).is_ok();
            if has_machete {
                commands::execute("cargo-machete", &[])?;
            } else {
                println!("  ! cargo-machete not found, skipping");
            }
        }
        QualityCheck::Msrv => {
            if Path::new("scripts/audit-msrv.sh").exists() {
                commands::execute("bash", &["scripts/audit-msrv.sh"])?;
            } else {
                println!("  ! scripts/audit-msrv.sh not found, skipping");
            }
        }
        QualityCheck::ShellCheck => {
            let has_shellcheck = commands::execute_captured("shellcheck", &["--version"]).is_ok();
            if has_shellcheck {
                let mut sh_files = Vec::new();
                find_files(Path::new("."), "sh", &mut sh_files);
                if sh_files.is_empty() {
                    println!("  ✓ No shell scripts detected");
                } else {
                    let mut args = vec!["--severity=error"];
                    let sh_strs: Vec<String> = sh_files.iter().map(|p| p.to_string_lossy().into_owned()).collect();
                    for s in &sh_strs {
                        args.push(s);
                    }
                    commands::execute("shellcheck", &args)?;
                }
            } else {
                println!("  ! shellcheck not found, skipping");
            }
        }
        QualityCheck::MarkdownLint => {
            let has_mdlint = commands::execute_captured("markdownlint-cli2", &["--version"]).is_ok();
            if has_mdlint {
                commands::execute("markdownlint-cli2", &["**/*.md"])?;
            } else {
                println!("  ! markdownlint-cli2 not found, skipping");
            }
        }
        QualityCheck::PrivacyCheck => {
            run_privacy_check()?;
        }
        QualityCheck::SecretScan => {
            run_secret_scan()?;
        }
        QualityCheck::WorkflowValidation => {
            if Path::new("scripts/validate-workflows.sh").exists() {
                commands::execute("bash", &["scripts/validate-workflows.sh"])?;
            } else {
                println!("  ! scripts/validate-workflows.sh not found, skipping");
            }
        }
        QualityCheck::SkillEvals => {
            if Path::new("scripts/run-evals.sh").exists() {
                commands::execute("bash", &["scripts/run-evals.sh"])?;
            } else {
                println!("  ! scripts/run-evals.sh not found, skipping");
            }
        }
        QualityCheck::LlmContext => {
            let files = &["llms.txt", "llms-full.txt"];
            for f in files {
                if !Path::new(f).exists() {
                    return Err(XtaskError::InvalidConfig {
                        message: format!("LLM context file '{f}' missing. Run scripts/generate-llms-txt.sh"),
                    });
                }
            }
            println!("  ✓ llms.txt and llms-full.txt are present");
        }
        QualityCheck::CiStatusArtifact => {
            let path = Path::new(".agents/ci/ci-status.json");
            if path.exists() {
                let mut file = File::open(path).map_err(|e| XtaskError::InvalidConfig { message: e.to_string() })?;
                let mut content = String::new();
                file.read_to_string(&mut content).map_err(|e| XtaskError::InvalidConfig { message: e.to_string() })?;
                let _v: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
                    XtaskError::InvalidConfig {
                        message: format!("CI status artifact is invalid JSON: {e}"),
                    }
                })?;
                println!("  ✓ CI status artifact exists and is valid JSON");
            } else {
                println!("  ! CI status artifact .agents/ci/ci-status.json not found, skipping check");
            }
        }
        QualityCheck::RoastScorer => {
            if Path::new("scripts/roast-scorer.sh").exists() {
                commands::execute("bash", &["scripts/roast-scorer.sh"])?;
            } else {
                println!("  ! scripts/roast-scorer.sh not found, skipping");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_checks_fast_pr() {
        let config = XtaskConfig::default();
        let checks = plan_checks(&config, Some("fast-pr"), None, None).unwrap();
        assert!(checks.contains(&QualityCheck::LocLimits));
        assert!(checks.contains(&QualityCheck::Fmt));
        assert!(checks.contains(&QualityCheck::Test));
        assert!(!checks.contains(&QualityCheck::ShellCheck));
    }

    #[test]
    fn test_plan_checks_only() {
        let config = XtaskConfig::default();
        let checks = plan_checks(&config, Some("fast-pr"), Some("fmt,clippy"), None).unwrap();
        assert_eq!(checks.len(), 2);
        assert!(checks.contains(&QualityCheck::Fmt));
        assert!(checks.contains(&QualityCheck::Clippy));
    }
}
