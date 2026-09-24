//! Xtask Configuration and Custom Errors.

use crate::quality::QualityCheck;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use thiserror::Error;

/// Clear error categories as required by implementation requirements.
#[derive(Debug, Error)]
pub enum XtaskError {
    /// Missing tool with installation guidance.
    #[error("Missing tool: '{tool_name}'. Guidance: {guidance}")]
    MissingTool {
        /// The name of the missing tool.
        tool_name: String,
        /// Instructions on how to install it.
        guidance: String,
    },

    /// Invalid configuration file or parameters.
    #[error("Invalid config: {message}")]
    InvalidConfig {
        /// Detail about why the config is invalid.
        message: String,
    },

    /// Unsupported platform for a tool or operation.
    #[error("Unsupported platform: {platform}")]
    UnsupportedPlatform {
        /// The name of the unsupported platform.
        platform: String,
    },

    /// Command execution failure.
    #[error("Command '{command}' failed with exit code: {exit_code:?}")]
    CommandFailure {
        /// The command that failed.
        command: String,
        /// The optional exit status/code of the command.
        exit_code: Option<i32>,
    },

    /// Issue accessing or managing cache.
    #[error("Cache issue: {message}")]
    CacheIssue {
        /// Detail about the cache issue.
        message: String,
    },
}

/// Resolves legacy tier aliases to their canonical tier names.
///
/// Kept next to the config types because both `quality::plan_checks` and
/// config validation must agree on the alias map.
#[must_use]
pub fn canonical_tier_name(selected: &str) -> &str {
    match selected {
        "fast-pr" => "pull-request",
        "full-gate" | "all" => "protected-branch",
        other => other,
    }
}

/// Lint-related thresholds.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct LintThresholds {
    /// Max lines of code per file.
    pub max_lines_per_file: usize,
    /// Treat Clippy warnings as errors.
    pub clippy_warnings_as_errors: bool,
}

/// Definition of one verification tier: the ordered set of checks it runs.
///
/// Tiers are the portable way to say *which* checks belong to *which* lifecycle
/// trigger (pull request, protected branch, scheduled run, release) without
/// embedding project-specific names or magic values in workflow YAML.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct TierDef {
    /// Quality checks this tier runs, in execution order.
    pub checks: Vec<QualityCheck>,
    /// Checks whose tool/script must be present for the tier to run: a required
    /// check whose tool is missing fails the gate with `XtaskError::MissingTool`
    /// instead of being skipped. When omitted, the built-in policy applies
    /// (the `protected-branch` and `release` tiers require the
    /// security/dependency checks; every other tier stays advisory). An
    /// explicit empty list is a deliberate opt-out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_checks: Option<Vec<QualityCheck>>,
}

/// Converts a glob pattern (supporting `**`, `*`, `?`) to a regex `String`.
#[must_use]
pub fn glob_to_regex_pattern(glob: &str) -> String {
    let mut regex = String::from("^");
    let chars: Vec<char> = glob.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '*' {
            if i + 1 < chars.len() && chars[i + 1] == '*' {
                i += 2;
                if i < chars.len() && chars[i] == '/' {
                    i += 1;
                    regex.push_str("(?:^|.*/)?");
                } else {
                    regex.push_str(".*");
                }
            } else {
                i += 1;
                regex.push_str("[^/]*");
            }
        } else if chars[i] == '?' {
            i += 1;
            regex.push_str("[^/]");
        } else {
            let c = chars[i];
            i += 1;
            if matches!(
                c,
                '.' | '+' | '(' | ')' | '{' | '}' | '[' | ']' | '^' | '$' | '|' | '\\'
            ) {
                regex.push('\\');
            }
            regex.push(c);
        }
    }
    regex.push('$');
    regex
}

/// Checks whether a file path matches a glob pattern.
#[must_use]
pub fn glob_match(pattern: &str, path: &str) -> bool {
    let re_str = glob_to_regex_pattern(pattern);
    regex::Regex::new(&re_str).is_ok_and(|re| re.is_match(path))
}

/// The main strongly typed configuration structure.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct XtaskConfig {
    /// Name of env variable that can override the default quality tier (e.g. "`XTASK_TIER`").
    pub env_var_name: String,
    /// Default quality tier to run if neither `--tier` nor the env override is set (e.g. "protected-branch").
    pub default_tier: String,
    /// Named verification tiers (e.g. "pull-request", "protected-branch"). Falls back to
    /// built-in defaults for names not present here, so a minimal config still works.
    #[serde(default)]
    pub tiers: BTreeMap<String, TierDef>,
    /// Declarative path->check glob mapping for when-changed check selection.
    #[serde(default)]
    pub when_changed: BTreeMap<QualityCheck, Vec<String>>,
    /// Configurable thresholds.
    pub lint_thresholds: LintThresholds,
}

impl Default for XtaskConfig {
    fn default() -> Self {
        Self {
            env_var_name: "XTASK_TIER".to_string(),
            default_tier: "protected-branch".to_string(),
            tiers: Self::builtin_tiers(),
            when_changed: Self::builtin_when_changed(),
            lint_thresholds: LintThresholds {
                max_lines_per_file: 500,
                clippy_warnings_as_errors: true,
            },
        }
    }
}

impl XtaskConfig {
    /// The portable, project-agnostic tier sets. Named after lifecycle triggers, not after any
    /// specific repository: adopters add or redefine tiers in `config/xtask.json` without
    /// touching workflow YAML.
    fn builtin_tiers() -> BTreeMap<String, TierDef> {
        use QualityCheck as Q;
        let mut tiers = BTreeMap::new();
        // Fast correctness gate for every pull request (no external security tooling required).
        tiers.insert(
            "pull-request".to_string(),
            TierDef {
                checks: vec![
                    Q::LocLimits,
                    Q::Fmt,
                    Q::Clippy,
                    Q::Build,
                    Q::Test,
                    Q::DocTest,
                    Q::PrivacyCheck,
                    Q::SecretScan,
                    Q::WorkflowValidation,
                ],
                required_checks: None,
            },
        );
        // Deep merge gate for protected branches: security/dependency policy plus the PR tier.
        tiers.insert(
            "protected-branch".to_string(),
            TierDef {
                checks: vec![
                    Q::LocLimits,
                    Q::Fmt,
                    Q::Clippy,
                    Q::Build,
                    Q::Test,
                    Q::DocTest,
                    Q::Audit,
                    Q::Deny,
                    Q::Machete,
                    Q::Msrv,
                    Q::ShellCheck,
                    Q::MarkdownLint,
                    Q::PrivacyCheck,
                    Q::SecretScan,
                    Q::WorkflowValidation,
                    Q::CiStatusArtifact,
                ],
                required_checks: None,
            },
        );
        // Expensive / repo-specific checks that only make sense on a schedule.
        tiers.insert(
            "scheduled".to_string(),
            TierDef {
                checks: vec![
                    Q::SkillValidation,
                    Q::AdrCompliance,
                    Q::SkillEvals,
                    Q::RoastScorer,
                    Q::LlmContext,
                    Q::Clippy,
                    Q::Test,
                    Q::Audit,
                    Q::Deny,
                    Q::Machete,
                    Q::Msrv,
                ],
                required_checks: None,
            },
        );
        // Pre-release gate.
        tiers.insert(
            "release".to_string(),
            TierDef {
                checks: vec![
                    Q::Clippy,
                    Q::Build,
                    Q::Test,
                    Q::DocTest,
                    Q::Audit,
                    Q::Deny,
                    Q::Machete,
                    Q::Msrv,
                    Q::WorkflowValidation,
                    Q::PrivacyCheck,
                    Q::SecretScan,
                ],
                required_checks: None,
            },
        );
        tiers
    }

    /// Built-in default path->check glob mappings.
    fn builtin_when_changed() -> BTreeMap<QualityCheck, Vec<String>> {
        use QualityCheck as Q;
        let rust_globs = vec![
            "crates/**/*.rs".to_string(),
            "src/**/*.rs".to_string(),
            "examples/**/*.rs".to_string(),
            "tests/**/*.rs".to_string(),
            "benchmarks/**/*.rs".to_string(),
            "fuzz/**/*.rs".to_string(),
            "Cargo.toml".to_string(),
            "Cargo.lock".to_string(),
            "rust-toolchain.toml".to_string(),
        ];
        let mut map = BTreeMap::new();
        map.insert(Q::Fmt, rust_globs.clone());
        map.insert(Q::Clippy, rust_globs.clone());
        map.insert(Q::Build, rust_globs.clone());
        map.insert(Q::Test, rust_globs.clone());
        map.insert(Q::DocTest, rust_globs);
        map.insert(
            Q::Audit,
            vec![
                "Cargo.toml".to_string(),
                "Cargo.lock".to_string(),
                "deny.toml".to_string(),
                ".cargo/audit.toml".to_string(),
            ],
        );
        map.insert(
            Q::Deny,
            vec![
                "Cargo.toml".to_string(),
                "Cargo.lock".to_string(),
                "deny.toml".to_string(),
            ],
        );
        map.insert(
            Q::Machete,
            vec![
                "crates/**/*.rs".to_string(),
                "src/**/*.rs".to_string(),
                "examples/**/*.rs".to_string(),
                "tests/**/*.rs".to_string(),
                "Cargo.toml".to_string(),
                "Cargo.lock".to_string(),
            ],
        );
        map.insert(
            Q::Msrv,
            vec![
                "crates/**/*.rs".to_string(),
                "src/**/*.rs".to_string(),
                "Cargo.toml".to_string(),
                "Cargo.lock".to_string(),
                "rust-toolchain.toml".to_string(),
                "scripts/audit-msrv.sh".to_string(),
            ],
        );
        map.insert(
            Q::ShellCheck,
            vec!["scripts/**/*.sh".to_string(), "**/*.sh".to_string()],
        );
        map.insert(Q::MarkdownLint, vec!["**/*.md".to_string()]);
        map.insert(
            Q::WorkflowValidation,
            vec![
                ".github/workflows/*.yml".to_string(),
                "scripts/validate-workflows.sh".to_string(),
            ],
        );
        map
    }
}

impl XtaskConfig {
    /// Loads the configuration from the specified path with fail-closed semantics.
    ///
    /// - **Missing file**: returns the built-in defaults. This is deliberate and
    ///   documented: `config/xtask.json` ships with the repository and with every
    ///   generated template project, so absence means the tool was invoked
    ///   outside a project root. The built-in tiers equal the shipped
    ///   configuration, so no check coverage is lost. Callers MUST NOT extend
    ///   this fallback to any present-but-invalid case.
    /// - **Present but unreadable, unparsable, or structurally invalid**: returns
    ///   `XtaskError::InvalidConfig`. Substituting defaults here would silently
    ///   revert repository-specific policy (required checks vanish while CI
    ///   stays green) — a security fail-open. Callers must exit nonzero.
    ///
    /// Also enforces file input limit to mitigate resource exhaustions.
    ///
    /// # Errors
    /// Returns `XtaskError::InvalidConfig` if the file exists but cannot be
    /// read, parsed, or validated.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, XtaskError> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        if !path.is_file() {
            return Err(XtaskError::InvalidConfig {
                message: format!("Config path '{}' is not a file", path.display()),
            });
        }
        let file = File::open(path).map_err(|e| XtaskError::InvalidConfig {
            message: format!("Failed to open config file: {e}"),
        })?;
        // Enforce input size limit (take max 1MB for safety)
        let mut handle = file.take(1_048_576);
        let mut content = String::new();
        handle
            .read_to_string(&mut content)
            .map_err(|e| XtaskError::InvalidConfig {
                message: format!("Failed to read config file: {e}"),
            })?;

        let mut config: Self =
            serde_json::from_str(&content).map_err(|e| XtaskError::InvalidConfig {
                message: format!("Failed to parse config JSON: {e}"),
            })?;
        // A config that omits `tiers` (serializer default = empty) must still get the portable
        // built-in tier sets so a minimal config keeps working.
        if config.tiers.is_empty() {
            config.tiers = Self::builtin_tiers();
        }
        config.validate()?;
        Ok(config)
    }

    /// Structural validation enforcing the fail-closed configuration contract:
    ///
    /// 1. `default_tier` (after legacy-alias resolution) names a defined tier;
    /// 2. every tier runs at least one check;
    /// 3. no tier repeats a check;
    /// 4. check names are known variants — enforced by serde during
    ///    deserialization, so an unknown name can never reach this point;
    /// 5. a tier's `required_checks` are a subset of its `checks`.
    ///
    /// # Errors
    /// Returns `XtaskError::InvalidConfig` describing the first violation.
    pub fn validate(&self) -> Result<(), XtaskError> {
        let default_canonical = canonical_tier_name(&self.default_tier);
        if !self.tiers.contains_key(default_canonical) {
            let defined: Vec<&str> = self.tiers.keys().map(String::as_str).collect();
            return Err(XtaskError::InvalidConfig {
                message: format!(
                    "default_tier '{default_canonical}' does not name a defined tier (defined: {defined:?})"
                ),
            });
        }
        for (name, def) in &self.tiers {
            if def.checks.is_empty() {
                return Err(XtaskError::InvalidConfig {
                    message: format!("tier '{name}' defines no checks"),
                });
            }
            let unique: BTreeSet<QualityCheck> = def.checks.iter().copied().collect();
            if unique.len() != def.checks.len() {
                return Err(XtaskError::InvalidConfig {
                    message: format!("tier '{name}' lists duplicate checks"),
                });
            }
            if let Some(required) = &def.required_checks {
                let unlisted: Vec<&str> = required
                    .iter()
                    .filter(|check| !unique.contains(*check))
                    .map(|check| check.name())
                    .collect();
                if !unlisted.is_empty() {
                    return Err(XtaskError::InvalidConfig {
                        message: format!(
                            "tier '{name}' requires check(s) [{}] that are not in its checks list",
                            unlisted.join(", ")
                        ),
                    });
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "config_test.rs"]
mod config_test;
