//! Changed paths detection and classification since a reference commit.

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

impl ChangedPaths {
    /// Classifies a list of file paths.
    pub fn classify<S: AsRef<str>>(files: &[S]) -> Self {
        let mut result = Self::default();
        result.changed_files = files.iter().map(|s| s.as_ref().to_string()).collect();

        for file in files {
            let path = file.as_ref();
            if path.is_empty() {
                continue;
            }

            // Detect Markdown
            if path.ends_with(".md") {
                result.has_markdown_changes = true;
            }

            // Detect Shell
            if path.ends_with(".sh") {
                result.has_shell_changes = true;
            }

            // Workflows
            if path.starts_with(".github/workflows/") {
                result.has_workflow_changes = true;
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
                result.has_agent_changes = true;
            }

            // Code
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
                if path.ends_with(".rs")
                    || path == "Cargo.toml"
                    || path == "Cargo.lock"
                    || path == "rust-toolchain.toml"
                {
                    result.has_code_changes = true;
                }
            }

            // Heavy
            if path.starts_with("crates/")
                || path.starts_with("src/")
                || path.starts_with("examples/")
                || path.starts_with("benchmarks/")
                || path.starts_with("fuzz/")
                || path == "Cargo.toml"
                || path == "Cargo.lock"
                || path == "rust-toolchain.toml"
            {
                result.has_heavy_changes = true;
            }
        }

        result
    }

    /// Queries git to retrieve changed files since the given base SHA, then classifies them.
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
                command: format!("git diff --name-only {base_sha}").to_string(),
                exit_code: output.status.code(),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let files: Vec<&str> = stdout
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect();

        Ok(Self::classify(&files))
    }
}

#[cfg(test)]
mod tests {
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
