//! Xtask Configuration and Custom Errors.

use serde::{Deserialize, Serialize};
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

/// The main strongly typed configuration structure.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct XtaskConfig {
    /// Default execution timeout in seconds.
    pub timeout_seconds: u64,
    /// Level of execution parallelism (number of concurrent tasks).
    pub parallelism: usize,
    /// Maximum command retries.
    pub retries: usize,
    /// Name of env variable to override config or tiers (e.g. "`XTASK_TIER`").
    pub env_var_name: String,
    /// Default quality tier to run if none is specified (e.g. "fast-pr").
    pub default_tier: String,
    /// Crate/package names in this workspace.
    pub package_names: Vec<String>,
    /// Configurable thresholds.
    pub lint_thresholds: LintThresholds,
}

impl Default for XtaskConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 600,
            parallelism: 1,
            retries: 0,
            env_var_name: "XTASK_TIER".to_string(),
            default_tier: "fast-pr".to_string(),
            package_names: vec![
                "actor-runtime-template".to_string(),
                "checkpoint-template".to_string(),
                "example-crate".to_string(),
                "example-registry-pattern".to_string(),
                "example-storage-pattern".to_string(),
                "hybrid-storage-template".to_string(),
                "mcp-server-template".to_string(),
                "sample-app".to_string(),
                "workspace-tests".to_string(),
                "xtask".to_string(),
            ],
            lint_thresholds: LintThresholds {
                max_lines_per_file: 500,
                clippy_warnings_as_errors: true,
            },
        }
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

        let config: Self = serde_json::from_str(&content).map_err(|e| {
            XtaskError::InvalidConfig {
                message: format!("Failed to parse config JSON: {e}"),
            }
        })?;
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
        assert_eq!(config.timeout_seconds, 600);
        assert_eq!(config.lint_thresholds.max_lines_per_file, 500);
    }

    #[test]
    fn test_load_non_existent_file() {
        let result = XtaskConfig::load_from_file("non-existent-file.json").unwrap();
        assert_eq!(result.timeout_seconds, 600);
    }

    #[test]
    fn test_load_valid_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("valid-xtask-config.json");
        let mut file = File::create(&path).unwrap();
        file.write_all(b"{\"timeout_seconds\":120,\"parallelism\":2,\"retries\":1,\"env_var_name\":\"TEST_TIER\",\"default_tier\":\"full-gate\",\"package_names\":[],\"lint_thresholds\":{\"max_lines_per_file\":300,\"clippy_warnings_as_errors\":false}}").unwrap();

        let result = XtaskConfig::load_from_file(&path).unwrap();
        assert_eq!(result.timeout_seconds, 120);
        assert_eq!(result.parallelism, 2);
        assert_eq!(result.lint_thresholds.max_lines_per_file, 300);
        assert!(!result.lint_thresholds.clippy_warnings_as_errors);

        let _ = std::fs::remove_file(path);
    }
}
