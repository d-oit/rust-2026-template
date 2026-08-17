//! Quality check execution reporting (console, JSON, and GitHub Actions).

use crate::config::XtaskError;
use serde::{Deserialize, Serialize};
use std::fs::{File, create_dir_all};
use std::io::Write as _;
use std::path::Path;

/// Individual check result.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct CheckResult {
    /// Name of the check.
    pub name: String,
    /// Status: "success", "failed", "skipped".
    pub status: String,
    /// Optional additional details.
    pub message: Option<String>,
}

/// Consolidated quality gate report.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct QualityReport {
    /// Timestamp of execution.
    pub timestamp: String,
    /// Git commit SHA.
    pub commit: String,
    /// Git branch.
    pub branch: String,
    /// Individual check execution results.
    pub checks: Vec<CheckResult>,
    /// Overall execution status: "success", "failure".
    pub overall: String,
}

impl QualityReport {
    /// Prints the report to the local console beautifully.
    pub fn print_console(&self) {
        println!("=================================================================");
        println!("│                    QUALITY GATE REPORT                        │");
        println!("=================================================================");
        for check in &self.checks {
            let symbol = match check.status.as_str() {
                "success" => "✓",
                "failed" => "✗",
                _ => "!",
            };
            println!(
                "  {symbol} {:<40} : {}",
                check.name,
                check.status.to_uppercase()
            );
        }
        println!("=================================================================");
        if self.overall == "success" {
            println!("  Overall Status: SUCCESS");
        } else {
            println!("  Overall Status: FAILURE");
        }
        println!("=================================================================");
    }

    /// Writes the report to `GITHUB_STEP_SUMMARY` or a specified file as markdown.
    ///
    /// # Errors
    /// Returns `XtaskError::CacheIssue` if the file cannot be written.
    pub fn write_github_summary(&self) -> Result<(), XtaskError> {
        use std::fmt::Write as _;

        let mut markdown = String::new();
        let _ = writeln!(markdown, "# Quality Gate Run Summary\n");
        let _ = writeln!(markdown, "**Timestamp:** {}", self.timestamp);
        let _ = writeln!(markdown, "**Commit:** {}", self.commit);
        let _ = writeln!(markdown, "**Branch:** {}\n", self.branch);
        let _ = writeln!(markdown, "| Check | Status | Details |");
        let _ = writeln!(markdown, "|---|---|---|");
        for check in &self.checks {
            let emoji = match check.status.as_str() {
                "success" => "✅ success",
                "failed" => "❌ failed",
                _ => "⚠️ skipped",
            };
            let details = check.message.as_deref().unwrap_or("");
            let _ = writeln!(markdown, "| {} | {emoji} | {details} |", check.name);
        }
        let overall_upper = self.overall.to_uppercase();
        let _ = writeln!(markdown, "\n### Overall: **{overall_upper}**");

        // Write to $GITHUB_STEP_SUMMARY if exists
        if let Ok(summary_path) = std::env::var("GITHUB_STEP_SUMMARY") {
            if !summary_path.is_empty() {
                let mut file = std::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(&summary_path)
                    .map_err(|e| XtaskError::CacheIssue {
                        message: e.to_string(),
                    })?;
                writeln!(file, "\n{markdown}").map_err(|e| XtaskError::CacheIssue {
                    message: e.to_string(),
                })?;
                println!("  ✓ Appended summary to $GITHUB_STEP_SUMMARY");
            }
        }

        // Write to .agents/ci/ci-summary.md
        let summary_dir = Path::new(".agents/ci");
        if let Err(e) = create_dir_all(summary_dir) {
            println!("  ! Warning: could not create .agents/ci directory: {e}");
        } else {
            let summary_file = summary_dir.join("ci-summary.md");
            let mut file = File::create(&summary_file).map_err(|e| XtaskError::CacheIssue {
                message: e.to_string(),
            })?;
            file.write_all(markdown.as_bytes())
                .map_err(|e| XtaskError::CacheIssue {
                    message: e.to_string(),
                })?;
            println!("  ✓ Wrote summary markdown to {}", summary_file.display());
        }

        Ok(())
    }

    /// Writes the structured JSON report to `.agents/ci/ci-status.json` and `reports/quality-report.json`.
    ///
    /// # Errors
    /// Returns `XtaskError` if writing files fails.
    pub fn write_json_report(&self) -> Result<(), XtaskError> {
        let json_str =
            serde_json::to_string_pretty(self).map_err(|e| XtaskError::InvalidConfig {
                message: e.to_string(),
            })?;

        let report_dir = Path::new(".agents/ci");
        if let Err(e) = create_dir_all(report_dir) {
            println!("  ! Warning: could not create .agents/ci directory: {e}");
        } else {
            let report_file = report_dir.join("ci-status.json");
            let mut file = File::create(&report_file).map_err(|e| XtaskError::CacheIssue {
                message: e.to_string(),
            })?;
            file.write_all(json_str.as_bytes())
                .map_err(|e| XtaskError::CacheIssue {
                    message: e.to_string(),
                })?;
            println!("  ✓ Wrote JSON report to {}", report_file.display());
        }

        let general_report_dir = Path::new("reports");
        if let Err(e) = create_dir_all(general_report_dir) {
            println!("  ! Warning: could not create reports directory: {e}");
        } else {
            let report_file = general_report_dir.join("quality-report.json");
            let mut file = File::create(&report_file).map_err(|e| XtaskError::CacheIssue {
                message: e.to_string(),
            })?;
            file.write_all(json_str.as_bytes())
                .map_err(|e| XtaskError::CacheIssue {
                    message: e.to_string(),
                })?;
            println!("  ✓ Wrote JSON report to {}", report_file.display());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn test_report_overall_success() {
        let report = QualityReport {
            timestamp: "2026-06-01T00:00:00Z".to_string(),
            commit: "abc1234".to_string(),
            branch: "main".to_string(),
            checks: vec![CheckResult {
                name: "Rust Format".to_string(),
                status: "success".to_string(),
                message: None,
            }],
            overall: "success".to_string(),
        };

        assert_eq!(report.overall, "success");
        assert_eq!(report.checks.len(), 1);
        assert_eq!(report.checks[0].status, "success");
    }
}
