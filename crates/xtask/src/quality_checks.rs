//! Individual quality check executors.
//!
//! Split out of `quality.rs` so both files stay within the 500-line limit
//! enforced by the LOC check. `quality.rs` owns check selection (which checks
//! run for a tier); this module owns how a selected check executes.

use crate::commands;
use crate::config::{XtaskConfig, XtaskError};
use crate::quality::QualityCheck;
use crate::quality_helpers;
use crate::quality_policy;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub(super) fn run_loc_limits_check(config: &XtaskConfig) -> Result<(), XtaskError> {
    let max_lines = config.lint_thresholds.max_lines_per_file;
    let mut rs_files = Vec::new();
    quality_helpers::find_files(Path::new("."), "rs", &mut rs_files);
    if rs_files.is_empty() {
        return Err(XtaskError::InvalidConfig {
            message: "LOC check failed: no Rust source files (.rs) found in workspace".to_string(),
        });
    }
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

pub(super) fn run_fmt_check() -> Result<(), XtaskError> {
    commands::execute("cargo", &["fmt", "--all", "--", "--check"])
}

pub(super) fn run_clippy_check(config: &XtaskConfig) -> Result<(), XtaskError> {
    let mut clippy_args = vec!["clippy", "--workspace", "--all-targets", "--all-features"];
    if config.lint_thresholds.clippy_warnings_as_errors {
        clippy_args.extend(&["--", "-D", "warnings"]);
    }
    commands::execute("cargo", &clippy_args)
}

pub(super) fn run_build_check() -> Result<(), XtaskError> {
    commands::execute("cargo", &["build", "--workspace", "--all-targets"])
}

pub(super) fn has_executed_tests(output: &str) -> bool {
    for line in output.lines() {
        if let Some(idx) = line.find("running ") {
            let rest = &line[idx + "running ".len()..];
            let num_str = rest.split_whitespace().next().unwrap_or("");
            if let Ok(count) = num_str.parse::<usize>() {
                if count > 0 {
                    return true;
                }
            }
        }
        if line.contains("passed") && !line.contains("0 passed") {
            return true;
        }
    }
    false
}

pub(super) fn run_test_check() -> Result<(), XtaskError> {
    let has_nextest = commands::execute_captured("cargo", &["nextest", "--version"]).is_ok();
    if has_nextest {
        commands::execute(
            "cargo",
            &[
                "nextest",
                "run",
                "--all-features",
                "--workspace",
                "--no-tests=fail",
            ],
        )
    } else {
        let output =
            commands::execute_captured("cargo", &["test", "--all-features", "--workspace"])?;
        if !has_executed_tests(&output) {
            return Err(XtaskError::CommandFailure {
                command: "cargo test (zero tests found in workspace)".to_string(),
                exit_code: Some(1),
            });
        }
        println!("{output}");
        Ok(())
    }
}

pub(super) fn run_doc_test() -> Result<(), XtaskError> {
    commands::execute("cargo", &["test", "--doc", "--all-features"])
}

pub(super) fn run_audit_check(config: &XtaskConfig) -> Result<(), XtaskError> {
    quality_policy::run_with_tool_policy(config, QualityCheck::Audit, || {
        commands::execute("cargo", &["audit"])
    })
}

pub(super) fn run_deny_check(config: &XtaskConfig) -> Result<(), XtaskError> {
    quality_policy::run_with_tool_policy(config, QualityCheck::Deny, || {
        commands::execute("cargo", &["deny", "check"])
    })
}

pub(super) fn run_machete_check(config: &XtaskConfig) -> Result<(), XtaskError> {
    quality_policy::run_with_tool_policy(config, QualityCheck::Machete, || {
        commands::execute("cargo-machete", &[])
    })
}

pub(super) fn run_msrv_check(config: &XtaskConfig) -> Result<(), XtaskError> {
    quality_policy::run_with_tool_policy(config, QualityCheck::Msrv, || {
        commands::execute("bash", &["scripts/audit-msrv.sh"])
    })
}

pub(super) fn run_shellcheck_check(config: &XtaskConfig) -> Result<(), XtaskError> {
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

pub(super) fn run_markdownlint_check(config: &XtaskConfig) -> Result<(), XtaskError> {
    quality_policy::run_with_tool_policy(config, QualityCheck::MarkdownLint, || {
        commands::execute("markdownlint-cli2", &["**/*.md"])
    })
}

pub(super) fn run_workflow_validation(config: &XtaskConfig) -> Result<(), XtaskError> {
    quality_policy::run_with_tool_policy(config, QualityCheck::WorkflowValidation, || {
        commands::execute("bash", &["scripts/validate-workflows.sh"])
    })
}

/// Lint GitHub Actions workflows with `actionlint`.
///
/// Only actionlint's own semantic checks run. Its bundled shellcheck and
/// pyflakes passes are disabled: the repository lints shell separately via
/// the `ShellCheck` check (which scans `scripts/`), and actionlint's shellcheck
/// pass reports style-level findings on workflow `run:` blocks that are
/// intentional in this repository (for example `dist $CMD`, which relies on
/// word splitting).
pub(super) fn run_actionlint_check(config: &XtaskConfig) -> Result<(), XtaskError> {
    quality_policy::run_with_tool_policy(config, QualityCheck::ActionLint, || {
        let mut workflow_files = Vec::new();
        quality_helpers::find_files(Path::new(".github/workflows"), "yml", &mut workflow_files);
        workflow_files.sort();
        if workflow_files.is_empty() {
            println!("  ! No workflow files found under .github/workflows, skipping check");
            return Ok(());
        }
        let args: Vec<String> = workflow_files
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        let mut arg_refs: Vec<&str> = vec!["-shellcheck=", "-pyflakes="];
        arg_refs.extend(args.iter().map(String::as_str));
        commands::execute("actionlint", &arg_refs)
    })
}

pub(super) fn run_llm_context_check() -> Result<(), XtaskError> {
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
    commands::execute("bash", &["scripts/generate-llms-txt.sh", "--check"])?;
    println!("  ✓ llms.txt and llms-full.txt are present and up to date");
    Ok(())
}

pub(super) fn run_ci_status_check() -> Result<(), XtaskError> {
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
