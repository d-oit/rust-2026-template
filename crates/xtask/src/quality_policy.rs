//! Required-tool policy for quality checks: the fail-closed heart of the gate.
//!
//! Tiers that protect `main` and releases must never pass *because* a security
//! tool happened to be absent. This module centralizes:
//!
//! - requiredness resolution ([`required_checks`]): config-driven via a tier's
//!   `required_checks` list, with a built-in default policy for tiers that omit
//!   it (`protected-branch` and `release` require the security/dependency
//!   checks; every other tier stays advisory);
//! - the tool-presence decision ([`tool_present_with`]), with injectable probes
//!   so tests never spawn processes;
//! - the skip-vs-fail decision ([`absent_outcome`]): required + absent yields
//!   [`XtaskError::MissingTool`] with actionable guidance, optional + absent
//!   keeps the historical "not found, skipping" behavior.
//!
//! The planned tier is published by `quality::plan_checks` via
//! [`set_active_tier`] and consumed by `quality::run_check` at execution time,
//! so requiredness always matches the tier that was actually planned
//! (including `--tier` overrides and the `$XTASK_TIER` env override).

use crate::commands;
use crate::config::{TierDef, XtaskConfig, XtaskError};
use crate::quality::QualityCheck;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::path::Path;

/// Tiers that fail closed by default: they guard `main` merges and releases.
const DEFAULT_REQUIRED_TIERS: [&str; 2] = ["protected-branch", "release"];

/// Checks that must never be silently skipped on the default-required tiers.
const DEFAULT_REQUIRED_CHECKS: [QualityCheck; 8] = [
    QualityCheck::Audit,
    QualityCheck::Deny,
    QualityCheck::Machete,
    QualityCheck::Msrv,
    QualityCheck::ShellCheck,
    QualityCheck::MarkdownLint,
    QualityCheck::WorkflowValidation,
    QualityCheck::SecretScan,
];

thread_local! {
    /// Canonical tier name resolved by the most recent `plan_checks` call on
    /// this thread. `quality::run_check` consumes it so requiredness always
    /// matches the tier that was actually planned.
    static ACTIVE_TIER: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Publishes the canonical tier resolved during planning (called by the parent
/// `quality` module when `plan_checks` resolves a tier).
pub(super) fn set_active_tier(canonical_tier: &str) {
    ACTIVE_TIER.with(|slot| *slot.borrow_mut() = Some(canonical_tier.to_string()));
}

/// What a check needs from the environment to actually run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolRequirement {
    /// An executable on `PATH`, probed through its version flag.
    Binary {
        /// Executable probed (e.g. `cargo` for the `cargo audit --version` probe).
        tool: &'static str,
        /// Arguments making the probe succeed only when the tool exists.
        version_args: &'static [&'static str],
    },
    /// A version-controlled repository script, probed by existence.
    Script {
        /// Script path relative to the repository root.
        path: &'static str,
    },
}

/// The external dependency of one check, plus how to obtain it when missing.
struct CheckTool {
    /// Name shown to users (e.g. `cargo-audit`, not the `cargo` probe binary).
    tool_name: &'static str,
    /// How presence is probed.
    requirement: ToolRequirement,
    /// Installation/restoration guidance embedded in `MissingTool` errors.
    guidance: &'static str,
}

/// Maps a check to its external dependency, if any. Checks without an entry run
/// in-process (e.g. `SecretScan`) and can never be skipped for lack of a tool.
const fn check_tool(check: QualityCheck) -> Option<CheckTool> {
    let (tool_name, requirement, guidance) = match check {
        QualityCheck::Audit => (
            "cargo-audit",
            ToolRequirement::Binary {
                tool: "cargo",
                version_args: &["audit", "--version"],
            },
            "install with `cargo install cargo-audit --locked`",
        ),
        QualityCheck::Deny => (
            "cargo-deny",
            ToolRequirement::Binary {
                tool: "cargo",
                version_args: &["deny", "--version"],
            },
            "install with `cargo install cargo-deny --locked`",
        ),
        QualityCheck::Machete => (
            "cargo-machete",
            ToolRequirement::Binary {
                tool: "cargo-machete",
                version_args: &["--version"],
            },
            "install with `cargo install cargo-machete --locked`",
        ),
        QualityCheck::Msrv => (
            "scripts/audit-msrv.sh",
            ToolRequirement::Script {
                path: "scripts/audit-msrv.sh",
            },
            "restore the script from version control (it ships with the template)",
        ),
        QualityCheck::ShellCheck => (
            "shellcheck",
            ToolRequirement::Binary {
                tool: "shellcheck",
                version_args: &["--version"],
            },
            "install shellcheck (e.g. `apt-get install shellcheck` or `brew install shellcheck`)",
        ),
        QualityCheck::MarkdownLint => (
            "markdownlint-cli2",
            ToolRequirement::Binary {
                tool: "markdownlint-cli2",
                version_args: &["--version"],
            },
            "install with `npm install -g markdownlint-cli2`",
        ),
        QualityCheck::WorkflowValidation => (
            "scripts/validate-workflows.sh",
            ToolRequirement::Script {
                path: "scripts/validate-workflows.sh",
            },
            "restore the script from version control (it ships with the template)",
        ),
        _ => return None,
    };
    Some(CheckTool {
        tool_name,
        requirement,
        guidance,
    })
}

/// Pure requiredness resolution: which checks must not be skipped in `tier_name`.
///
/// An explicit `required_checks` list on the tier definition always wins
/// (including an explicit empty list as a deliberate opt-out). Otherwise the
/// built-in policy applies: the security/dependency checks are required on
/// `protected-branch` and `release`, advisory everywhere else.
#[must_use]
fn required_checks(tier_name: &str, def: &TierDef) -> BTreeSet<QualityCheck> {
    if let Some(explicit) = &def.required_checks {
        return explicit.iter().copied().collect();
    }
    if DEFAULT_REQUIRED_TIERS.contains(&tier_name) {
        DEFAULT_REQUIRED_CHECKS.into_iter().collect()
    } else {
        BTreeSet::new()
    }
}

/// Pure tool-presence decision with injectable probes (tests never spawn processes).
#[must_use]
fn tool_present_with(
    requirement: &ToolRequirement,
    probe_binary: impl Fn(&str, &[&str]) -> bool,
    script_exists: impl Fn(&str) -> bool,
) -> bool {
    match requirement {
        ToolRequirement::Binary { tool, version_args } => probe_binary(tool, version_args),
        ToolRequirement::Script { path } => script_exists(path),
    }
}

fn tool_present(requirement: &ToolRequirement) -> bool {
    tool_present_with(
        requirement,
        |tool, args| commands::execute_captured(tool, args).is_ok(),
        |path| Path::new(path).exists(),
    )
}

/// Decision when a check's tool/script is absent: required checks fail closed
/// with actionable guidance; optional checks keep the historical skip behavior.
fn absent_outcome(
    dependency: &CheckTool,
    required: bool,
    tier_name: &str,
) -> Result<(), XtaskError> {
    if required {
        return Err(XtaskError::MissingTool {
            tool_name: dependency.tool_name.to_string(),
            guidance: format!(
                "{} (required by tier '{tier_name}'; the gate fails closed instead of skipping)",
                dependency.guidance
            ),
        });
    }
    println!("  ! {} not found, skipping", dependency.tool_name);
    Ok(())
}

/// The tier requiredness is resolved against: the tier planned by `plan_checks`
/// on this thread, or — when planning never ran (direct `run_check` callers) —
/// the config default (env override wins), canonicalized.
fn active_or_fallback_tier(config: &XtaskConfig) -> String {
    ACTIVE_TIER
        .with(|slot| slot.borrow().clone())
        .unwrap_or_else(|| {
            let selected = std::env::var(&config.env_var_name)
                .ok()
                .unwrap_or_else(|| config.default_tier.clone());
            crate::config::canonical_tier_name(&selected).to_string()
        })
}

fn tier_required(config: &XtaskConfig, tier_name: &str, check: QualityCheck) -> bool {
    config
        .tiers
        .get(tier_name)
        .is_some_and(|def| required_checks(tier_name, def).contains(&check))
}

/// Requiredness of `check` for the active tier (test-visible decision point).
#[cfg(test)]
fn is_required(config: &XtaskConfig, check: QualityCheck) -> bool {
    tier_required(config, &active_or_fallback_tier(config), check)
}

/// Probes the check's tool/script and either runs the check, or applies the
/// required-tool policy for the active tier (called by the parent `quality`
/// module when executing a check with an external tool/script dependency).
pub(super) fn run_with_tool_policy<F>(
    config: &XtaskConfig,
    check: QualityCheck,
    run: F,
) -> Result<(), XtaskError>
where
    F: FnOnce() -> Result<(), XtaskError>,
{
    let Some(dependency) = check_tool(check) else {
        return run(); // no external dependency: nothing to fail closed about
    };
    if tool_present(&dependency.requirement) {
        return run();
    }
    let tier_name = active_or_fallback_tier(config);
    let required = tier_required(config, &tier_name, check);
    absent_outcome(&dependency, required, &tier_name)
}

#[cfg(test)]
#[path = "quality_policy_test.rs"]
mod quality_policy_test;
