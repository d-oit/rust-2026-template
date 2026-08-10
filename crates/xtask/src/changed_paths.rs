//! Changed paths detection and classification since a reference commit.
#![allow(clippy::struct_excessive_bools)]

use crate::config::XtaskError;
use serde::{Deserialize, Serialize};
use std::process::Command;

/// Classified changed paths and files.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangedPaths {
    /// True if code files (crates, src, Cargo.toml, rust-toolchain.toml, etc.) changed.
    pub has_code_changes: bool,
    /// True if heavy files (benchmarks, fuzz, etc.) changed.
    pub has_heavy_changes: bool,
    /// True if agent-related files or documentation changed.
    pub has_agent_changes: bool,
    /// True if GitHub Action workflow files changed.
    pub has_workflow_changes: bool,
    /// True if shell scripts (.sh) changed.
    pub has_shell_changes: bool,
    /// True if Markdown files (.md) changed.
    pub has_markdown_changes: bool,
    /// The actual files that changed.
    pub changed_files: Vec<String>,
}

/// Extracts the unique top-level crate names (`crates/<name>/…`) that contain changed files,
/// in first-seen order. Used by CI telemetry to report the affected-package scope.
#[must_use]
pub fn affected_crates(changed_files: &[String]) -> Vec<String> {
    let mut seen = Vec::new();
    for file in changed_files {
        if let Some(rest) = file.strip_prefix("crates/") {
            if let Some(crate_name) = rest.split('/').next() {
                if !crate_name.is_empty() && !seen.iter().any(|s| s == crate_name) {
                    seen.push(crate_name.to_string());
                }
            }
        }
    }
    seen
}

impl ChangedPaths {
    /// Classifies a list of file paths.
    pub fn classify<S: AsRef<str>>(files: &[S]) -> Self {
        let mut has_code_changes = false;
        let mut has_heavy_changes = false;
        let mut has_agent_changes = false;
        let mut has_workflow_changes = false;
        let mut has_shell_changes = false;
        let mut has_markdown_changes = false;

        for file in files {
            let path = file.as_ref();
            if path.is_empty() {
                continue;
            }

            // Detect Markdown
            if std::path::Path::new(path)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            {
                has_markdown_changes = true;
            }

            // Detect Shell
            if std::path::Path::new(path)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("sh"))
            {
                has_shell_changes = true;
            }

            // Workflows
            if path.starts_with(".github/workflows/") {
                has_workflow_changes = true;
            }

            // Agents
            if path == "AGENTS.md"
                || path == "CLAUDE.md"
                || path == "GEMINI.md"
                || path == "QWEN.md"
                || path.starts_with("agents-docs/")
                || path.starts_with(".agents/skills/")
                || path == ".agents/SKILLS.md"
                || path == "scripts/validate-agent-entrypoints.sh"
                || path == "scripts/generate-skills-md.sh"
            {
                has_agent_changes = true;
            }

            // Code and Heavy
            if path.starts_with("crates/")
                || path.starts_with("src/")
                || path.starts_with("examples/")
                || path.starts_with("tests/")
                || path.starts_with("benchmarks/")
                || path.starts_with("fuzz/")
                || path == "Cargo.toml"
                || path == "Cargo.lock"
                || path == "rust-toolchain.toml"
            {
                has_heavy_changes = true;
                let file_name = std::path::Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                let is_rust_source = std::path::Path::new(path)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("rs"));
                // Any manifest (root or subcrate) can change the dependency graph and
                // therefore the compiled artifact set — count it as a code change so
                // `--changed-from` keeps the compile/test chain for manifest-only PRs.
                let is_manifest = file_name == "Cargo.toml"
                    || file_name == "Cargo.lock"
                    || path == "rust-toolchain.toml";
                if is_rust_source || is_manifest {
                    has_code_changes = true;
                }
            }
        }

        Self {
            has_code_changes,
            has_heavy_changes,
            has_agent_changes,
            has_workflow_changes,
            has_shell_changes,
            has_markdown_changes,
            changed_files: files.iter().map(|s| s.as_ref().to_string()).collect(),
        }
    }

    /// Queries git to retrieve changed files since the given base SHA, then classifies them.
    ///
    /// # Errors
    /// Returns `XtaskError::CommandFailure` if the git command fails to run or execute.
    pub fn from_git(base_sha: &str) -> Result<Self, XtaskError> {
        let output = Command::new("git")
            .args(["diff", "--name-only", base_sha])
            .output()
            .map_err(|_e| XtaskError::CommandFailure {
                command: format!("git diff --name-only {base_sha}"),
                exit_code: None,
            })?;

        if !output.status.success() {
            return Err(XtaskError::CommandFailure {
                command: format!("git diff --name-only {base_sha}"),
                exit_code: output.status.code(),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let files: Vec<&str> = stdout
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();

        Ok(Self::classify(&files))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn test_classify_empty() {
        let empty: &[&str] = &[];
        let cp = ChangedPaths::classify(empty);
        assert!(!cp.has_code_changes);
        assert!(!cp.has_heavy_changes);
        assert!(!cp.has_agent_changes);
        assert!(!cp.has_workflow_changes);
        assert!(!cp.has_shell_changes);
        assert!(!cp.has_markdown_changes);
    }

    #[test]
    fn test_affected_crates_top_level() {
        let files = vec![
            "crates/xtask/src/main.rs".to_string(),
            "crates/xtask/Cargo.toml".to_string(),
            "crates/sample-app/src/lib.rs".to_string(),
            "Cargo.toml".to_string(),
        ];
        assert_eq!(affected_crates(&files), vec!["xtask", "sample-app"]);
    }

    #[test]
    fn test_classify_subcrate_manifest() {
        // A dependency manifest change (not just .rs) must count as a code change,
        // otherwise `--changed-from` would drop the compile/test chain for it.
        let files = vec!["crates/sample-app/Cargo.toml"];
        let cp = ChangedPaths::classify(&files);
        assert!(cp.has_code_changes);
        assert!(cp.has_heavy_changes);
    }

    #[test]
    fn test_classify_code() {
        let files = vec!["crates/sample-app/src/main.rs", "Cargo.toml"];
        let cp = ChangedPaths::classify(&files);
        assert!(cp.has_code_changes);
        assert!(cp.has_heavy_changes);
        assert!(!cp.has_agent_changes);
    }

    #[test]
    fn test_classify_agents() {
        let files = vec!["AGENTS.md", ".agents/skills/my_skill/SKILL.md"];
        let cp = ChangedPaths::classify(&files);
        assert!(cp.has_agent_changes);
        assert!(cp.has_markdown_changes);
        assert!(!cp.has_code_changes);
    }

    #[test]
    fn test_classify_workflows() {
        let files = vec![".github/workflows/ci.yml"];
        let cp = ChangedPaths::classify(&files);
        assert!(cp.has_workflow_changes);
        assert!(!cp.has_code_changes);
    }
}
