//! Xtask Configuration and Custom Errors.

use crate::quality::QualityCheck;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
    /// Configurable thresholds.
    pub lint_thresholds: LintThresholds,
}

impl Default for XtaskConfig {
    fn default() -> Self {
        Self {
            env_var_name: "XTASK_TIER".to_string(),
            default_tier: "protected-branch".to_string(),
            tiers: Self::builtin_tiers(),
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
                ],
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
            },
        );
        tiers
    }
}

impl XtaskConfig {
    /// Loads the configuration from the specified path, or returns defaults.
    /// Also enforces file input limit to mitigate resource exhaustions.
    ///
    /// # Errors
    /// Returns `XtaskError::InvalidConfig` if the file exists but cannot be read or parsed.
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
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use std::io::Write;

    #[test]
    fn test_default_config() {
        let config = XtaskConfig::default();
        assert_eq!(config.default_tier, "protected-branch");
        assert_eq!(config.env_var_name, "XTASK_TIER");
        assert_eq!(config.lint_thresholds.max_lines_per_file, 500);
        for tier in ["pull-request", "protected-branch", "scheduled", "release"] {
            assert!(
                config.tiers.contains_key(tier),
                "builtin tier {tier} must exist"
            );
        }
    }

    #[test]
    fn test_load_non_existent_file() {
        let result = XtaskConfig::load_from_file("non-existent-file.json").unwrap();
        assert_eq!(result.default_tier, "protected-branch");
        assert_eq!(result.tiers.len(), 4);
    }

    #[test]
    fn test_load_valid_file_without_tiers() {
        // A config without a `tiers` key still loads (backwards compatible via serde default).
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("valid-xtask-config.json");
        let mut file = File::create(&path).unwrap();
        file.write_all(b"{\"env_var_name\":\"TEST_TIER\",\"default_tier\":\"fast-pr\",\"lint_thresholds\":{\"max_lines_per_file\":300,\"clippy_warnings_as_errors\":false}}").unwrap();

        let result = XtaskConfig::load_from_file(&path).unwrap();
        assert_eq!(result.default_tier, "fast-pr");
        assert_eq!(result.env_var_name, "TEST_TIER");
        assert_eq!(result.lint_thresholds.max_lines_per_file, 300);
        assert!(!result.lint_thresholds.clippy_warnings_as_errors);
        // Falls back to built-in tiers.
        assert_eq!(result.tiers.len(), 4);
    }

    #[test]
    fn test_load_custom_tier_overrides_builtin() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("custom-tiers.json");
        std::fs::write(
            &path,
            r#"{"env_var_name":"XTASK_TIER","default_tier":"ci-smoke","tiers":{"ci-smoke":{"checks":["Fmt","Clippy"]}},"lint_thresholds":{"max_lines_per_file":500,"clippy_warnings_as_errors":true}}"#,
        )
        .unwrap();
        let result = XtaskConfig::load_from_file(&path).unwrap();
        assert_eq!(
            result.tiers["ci-smoke"].checks,
            vec![
                crate::quality::QualityCheck::Fmt,
                crate::quality::QualityCheck::Clippy
            ]
        );
    }
}
