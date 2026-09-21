//! Structured CI telemetry: emits `quality-run.json` and `quality-summary.md`.
//!
//! This is the portable, SaaS-free observability contract for the quality gate
//! (issue #289). Every `xtask quality run` writes a schema-versioned JSON artifact
//! plus a human-readable Markdown summary, so CI runs are debuggable without an
//! external monitoring stack.

use crate::commands;
use crate::config::XtaskError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write as _;
use std::path::Path;

/// Current telemetry artifact schema version (bump on any breaking field change).
pub const SCHEMA_VERSION: u32 = 2;

/// Freshness status of execution evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EvidenceStatus {
    /// Evidence exists, all checks passed, and fingerprint matches current workspace/policy state.
    Green,
    /// Evidence exists, but one or more checks failed.
    Red,
    /// Evidence exists, but workspace state or policy inputs have moved on.
    Stale,
    /// No evidence artifact was found.
    Missing,
}

impl std::fmt::Display for EvidenceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Green => write!(f, "GREEN"),
            Self::Red => write!(f, "RED"),
            Self::Stale => write!(f, "STALE"),
            Self::Missing => write!(f, "MISSING"),
        }
    }
}

/// Fingerprint embedding workspace state and policy hashes for freshness validation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceFingerprint {
    /// Git commit SHA (HEAD or `GITHUB_SHA`).
    pub head_commit: String,
    /// Hash of HEAD commit + uncommitted edits / untracked file status.
    pub worktree_hash: String,
    /// Quality gate tier used for the run.
    pub tier: String,
    /// Hash of policy inputs (e.g. `config/xtask.json`, `deny.toml`, workflow files).
    pub policy_hash: String,
}

/// Simple, deterministic FNV-1a 64-bit hashing function for string/bytes fingerprinting.
#[must_use]
pub fn fnv1a_hash(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// Computes the evidence fingerprint for the current workspace state and specified tier.
#[must_use]
pub fn compute_fingerprint(tier: &str) -> EvidenceFingerprint {
    let head_commit = std::env::var("GITHUB_SHA").unwrap_or_else(|_| {
        commands::execute_captured("git", &["rev-parse", "HEAD"])
            .unwrap_or_else(|_| "unknown_sha".to_string())
            .trim()
            .to_string()
    });

    let git_status =
        commands::execute_captured("git", &["status", "--porcelain"]).unwrap_or_default();
    let git_diff = commands::execute_captured("git", &["diff", "HEAD"]).unwrap_or_default();
    let mut worktree_bytes = Vec::new();
    worktree_bytes.extend_from_slice(head_commit.as_bytes());
    worktree_bytes.extend_from_slice(git_status.as_bytes());
    worktree_bytes.extend_from_slice(git_diff.as_bytes());
    let worktree_hash = format!("{:016x}", fnv1a_hash(&worktree_bytes));

    let policy_files = [
        "config/xtask.json",
        "deny.toml",
        "plans/dora.json",
        ".github/workflows/ci.yml",
        ".github/workflows/security-scan.yml",
    ];
    let mut policy_bytes = Vec::new();
    for file_path in policy_files {
        if let Ok(content) = fs::read(file_path) {
            policy_bytes.extend_from_slice(file_path.as_bytes());
            policy_bytes.extend_from_slice(&content);
        }
    }
    let policy_hash = format!("{:016x}", fnv1a_hash(&policy_bytes));

    EvidenceFingerprint {
        head_commit,
        worktree_hash,
        tier: tier.to_string(),
        policy_hash,
    }
}

/// Configurable telemetry behaviour (budgets are configuration, not application logic).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Master switch for emitting the telemetry artifact and summary.
    pub enabled: bool,
    /// Detail level: "minimal" or "full". "minimal" omits the per-stage cache/scope detail.
    pub detail: String,
    /// Days an emitted artifact should be retained by CI.
    pub retention_days: u32,
    /// Relative path of the Markdown summary.
    pub summary_path: String,
    /// Relative path of the JSON artifact.
    pub artifact_path: String,
    /// Execution budgets used to surface slow stages.
    pub budgets: TelemetryBudgets,
}

/// Stage-duration budgets surfaced in the summary when exceeded.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TelemetryBudgets {
    /// Per-stage wall-clock budget in milliseconds.
    pub max_stage_duration_ms: u64,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            detail: "full".to_string(),
            retention_days: 7,
            summary_path: ".agents/ci/quality-summary.md".to_string(),
            artifact_path: ".agents/ci/quality-run.json".to_string(),
            budgets: TelemetryBudgets {
                max_stage_duration_ms: 600_000,
            },
        }
    }
}

impl TelemetryConfig {
    /// Loads `config/ci/telemetry.toml`, falling back to defaults when absent or broken.
    #[must_use]
    pub fn load_or_default() -> Self {
        fn default_with_warning(reason: &dyn std::fmt::Display) -> TelemetryConfig {
            println!("  ! Warning: config/ci/telemetry.toml invalid ({reason}); using defaults");
            TelemetryConfig::default()
        }
        fs::read_to_string("config/ci/telemetry.toml").map_or_else(
            |_| Self::default(),
            |content| toml::from_str(&content).unwrap_or_else(|e| default_with_warning(&e)),
        )
    }
}

/// Decides which packages are in scope for the current run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TelemetryScope {
    /// "affected-packages" when a `--changed-from` base was supplied, else "all".
    pub mode: String,
    /// Unique crate names affected (top-level `crates/<name>/`), empty for "all".
    pub packages: Vec<String>,
    /// Whether the scope fell back to "all" (e.g. unresolved base or plan failure).
    pub fallback_used: bool,
}

/// Outcome of a single quality stage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TelemetryStage {
    /// Stage id (kebab-case check name).
    pub id: String,
    /// "passed", "failed", or "skipped".
    pub status: String,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
    /// Cache state for this stage: "restored", "saved", "miss", or "not-applicable".
    pub cache: String,
    /// Why the stage was skipped, when skipped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_reason: Option<String>,
}

/// Toolchain versions captured at run time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolchainInfo {
    /// `rustc --version` output.
    pub rustc: String,
    /// `cargo --version` output.
    pub cargo: String,
    /// `cargo nextest --version` output (or "unavailable" when only standard cargo test exists).
    pub nextest: String,
}

/// The full telemetry artifact emitted after a quality run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CiTelemetry {
    /// Artifact schema version (see [`SCHEMA_VERSION`]).
    pub schema_version: u32,
    /// ISO-8601 UTC timestamp of the run.
    pub timestamp: String,
    /// Selected tier (canonical name), e.g. "pull-request".
    pub tier: String,
    /// Where the tier plan came from, e.g. "config/xtask.json".
    pub plan_source: String,
    /// Changed-package scope decision for this run.
    pub scope: TelemetryScope,
    /// Executed and skipped stages with timing.
    pub stages: Vec<TelemetryStage>,
    /// Toolchain versions at run time.
    pub toolchain: ToolchainInfo,
    /// Evidence fingerprint matching workspace and policy state at execution time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<EvidenceFingerprint>,
}

/// Converts a human check name ("Rust Format") into a kebab-case stage id ("rust-format").
#[must_use]
pub fn stage_id(name: &str) -> String {
    name.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

impl ToolchainInfo {
    /// Captures current tool versions; missing tools are reported as "unavailable".
    #[must_use]
    pub fn capture() -> Self {
        fn first_line(cmd: &str, args: &[&str]) -> String {
            commands::execute_captured(cmd, args).map_or_else(
                |_| format!("{cmd}: unavailable"),
                |out| {
                    out.lines()
                        .next()
                        .unwrap_or("unavailable")
                        .trim()
                        .to_string()
                },
            )
        }
        Self {
            rustc: first_line("rustc", &["--version"]),
            cargo: first_line("cargo", &["--version"]),
            nextest: first_line("cargo", &["nextest", "--version"]),
        }
    }
}

impl CiTelemetry {
    /// Checks evidence freshness against the given expected fingerprint.
    #[must_use]
    pub fn check_freshness(&self, expected_fingerprint: &EvidenceFingerprint) -> EvidenceStatus {
        if self.stages.iter().any(|stage| stage.status == "failed") {
            return EvidenceStatus::Red;
        }
        match &self.fingerprint {
            Some(fp) if fp == expected_fingerprint => EvidenceStatus::Green,
            _ => EvidenceStatus::Stale,
        }
    }
    /// Writes the JSON artifact and the Markdown summary next to it.
    ///
    /// # Errors
    /// Returns `XtaskError::CacheIssue` if an artifact cannot be written.
    pub fn emit(&self, config: &TelemetryConfig) -> Result<(), XtaskError> {
        if !config.enabled {
            println!("  ! Telemetry disabled via config/ci/telemetry.toml");
            return Ok(());
        }

        // JSON artifact.
        let artifact = Path::new(&config.artifact_path);
        if let Some(parent) = artifact.parent() {
            fs::create_dir_all(parent).map_err(|e| XtaskError::CacheIssue {
                message: format!("Failed to create {}: {e}", parent.display()),
            })?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| XtaskError::InvalidConfig {
            message: e.to_string(),
        })?;
        let mut file = fs::File::create(artifact).map_err(|e| XtaskError::CacheIssue {
            message: e.to_string(),
        })?;
        file.write_all(json.as_bytes())
            .map_err(|e| XtaskError::CacheIssue {
                message: e.to_string(),
            })?;
        println!("  ✓ Wrote telemetry artifact to {}", artifact.display());

        // Markdown summary.
        let summary = Path::new(&config.summary_path);
        if let Some(parent) = summary.parent() {
            fs::create_dir_all(parent).map_err(|e| XtaskError::CacheIssue {
                message: format!("Failed to create {}: {e}", parent.display()),
            })?;
        }
        let mut file = fs::File::create(summary).map_err(|e| XtaskError::CacheIssue {
            message: e.to_string(),
        })?;
        file.write_all(self.summary_markdown(config).as_bytes())
            .map_err(|e| XtaskError::CacheIssue {
                message: e.to_string(),
            })?;
        println!("  ✓ Wrote telemetry summary to {}", summary.display());
        Ok(())
    }

    /// Renders the human-readable summary (also appended to the GHA step summary by CI).
    #[must_use]
    pub fn summary_markdown(&self, config: &TelemetryConfig) -> String {
        use std::fmt::Write as _;
        let mut md = String::new();
        let _ = writeln!(
            md,
            "## Quality Run Telemetry (schema v{})",
            self.schema_version
        );
        let _ = writeln!(md, "- **Tier:** {}", self.tier);
        let _ = writeln!(md, "- **Plan source:** {}", self.plan_source);
        if let Some(fp) = &self.fingerprint {
            let _ = writeln!(md, "- **Worktree Hash:** {}", fp.worktree_hash);
            let _ = writeln!(md, "- **Policy Hash:** {}", fp.policy_hash);
        }
        let _ = writeln!(md, "- **Scope:** {} {}", self.scope.mode, {
            if self.scope.packages.is_empty() {
                "(whole workspace)".to_string()
            } else {
                format!("({})", self.scope.packages.join(", "))
            }
        });
        if self.scope.fallback_used {
            let _ = writeln!(md, "- **Scope fallback:** used (base/plan unavailable)");
        }
        let _ = writeln!(md);
        let _ = writeln!(md, "| Stage | Status | Duration | Cache | Reason |");
        let _ = writeln!(md, "|---|---|---|---|---|");
        for stage in &self.stages {
            let emoji = match stage.status.as_str() {
                "passed" => "✅ passed",
                "failed" => "❌ failed",
                _ => "⏭️ skipped",
            };
            let reason = stage.skipped_reason.as_deref().unwrap_or("");
            let _ = writeln!(
                md,
                "| {} | {emoji} | {} ms | {} | {reason} |",
                stage.id, stage.duration_ms, stage.cache
            );
        }
        let _ = writeln!(md);
        let _ = writeln!(
            md,
            "- **Toolchain:** rustc={}, cargo={}, nextest={}",
            self.toolchain.rustc, self.toolchain.cargo, self.toolchain.nextest
        );
        if config.detail != "minimal" {
            let over = self
                .stages
                .iter()
                .filter(|s| s.duration_ms > config.budgets.max_stage_duration_ms)
                .map(|s| s.id.as_str())
                .collect::<Vec<_>>();
            if !over.is_empty() {
                let _ = writeln!(
                    md,
                    "- ⚠️ **Budget exceeded** (>{} ms): {}",
                    config.budgets.max_stage_duration_ms,
                    over.join(", ")
                );
            }
        }
        md
    }
}

#[cfg(test)]
#[path = "telemetry_test.rs"]
mod tests;
