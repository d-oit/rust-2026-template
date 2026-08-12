//! Agent adapter manifest validation (issue #287).
//!
//! Validates that tool-specific adapters (CLAUDE.md, GEMINI.md, etc.) remain thin
//! wrappers around the canonical `AGENTS.md` contract. The manifest at
//! `.agents/agent-adapters.toml` declares the expected adapter topology; this module
//! enforces it programmatically.

use crate::config::XtaskError;
use serde::Deserialize;
use std::fs;
use std::io::Read as _;
use std::path::Path;

/// Path to the adapter manifest, relative to the repository root.
pub const MANIFEST_PATH: &str = ".agents/agent-adapters.toml";

/// Maximum bytes to read from any adapter entrypoint file (1 MiB).
const MAX_ENTRYPOINT_BYTES: u64 = 1_048_576;

/// Maximum non-reference content lines before flagging an adapter as oversized.
const MAX_ADAPTER_BODY_LINES: usize = 40;

/// Canonical contract declaration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractConfig {
    /// Path to the canonical instructions file (e.g. "AGENTS.md").
    pub canonical_instructions: String,
    /// Path to the shared skills directory.
    pub skills_directory: String,
    /// Context files shipped for agent consumption.
    pub context_files: Vec<String>,
}

/// Validation rules.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
#[expect(clippy::struct_excessive_bools)]
pub struct ValidationConfig {
    /// Each adapter must reference the canonical instructions file.
    pub require_canonical_reference: bool,
    /// Reject adapters that duplicate canonical policy sections.
    pub reject_policy_duplication: bool,
    /// Referenced skills, hooks, and context files must exist on disk.
    pub verify_local_links: bool,
    /// Adapters are limited to tool-specific guidance.
    pub enforce_adapter_scope: bool,
    /// Maximum allowed line count for AGENTS.md.
    #[serde(default = "default_max_lines")]
    pub max_agents_md_lines: usize,
}

const fn default_max_lines() -> usize {
    200
}

/// A single registered adapter.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdapterConfig {
    /// Machine identifier (e.g. "claude").
    pub id: String,
    /// Root directory for this adapter's files (e.g. ".claude").
    pub root: String,
    /// Entrypoint file relative to the repository root (e.g. "CLAUDE.md").
    pub entrypoint: String,
    /// Adapter role (currently only "tool-delta").
    pub role: String,
    /// The canonical file this adapter must reference (e.g. "AGENTS.md").
    pub canonical_reference: String,
}

/// The full adapter manifest.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentAdaptersManifest {
    /// Canonical contract configuration.
    pub contract: ContractConfig,
    /// Validation rules.
    pub validation: ValidationConfig,
    /// Registered adapters.
    pub adapters: Vec<AdapterConfig>,
}

/// A single validation finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationFinding {
    /// Severity: "error" or "warning".
    pub severity: String,
    /// Adapter id or "contract" for global findings.
    pub source: String,
    /// Human-readable description.
    pub message: String,
}

/// Result of a full validation pass.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Errors that must be fixed.
    pub errors: Vec<ValidationFinding>,
    /// Warnings that should be reviewed.
    pub warnings: Vec<ValidationFinding>,
}

impl ValidationResult {
    /// Returns `true` when no errors were found.
    #[must_use]
    pub const fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    /// Prints findings to stdout.
    pub fn print_report(&self) {
        for err in &self.errors {
            println!("  ❌ [{}] {}", err.source, err.message);
        }
        for warn in &self.warnings {
            println!("  ⚠️  [{}] {}", warn.source, warn.message);
        }
        if self.is_ok() {
            println!("  ✅ All agent adapters validated successfully.");
        } else {
            println!(
                "  ❌ Validation failed with {} error(s) and {} warning(s).",
                self.errors.len(),
                self.warnings.len()
            );
        }
    }
}

/// Reads a file with a byte limit to prevent resource exhaustion.
fn read_bounded(path: &Path, max_bytes: u64) -> Result<String, XtaskError> {
    let file = fs::File::open(path).map_err(|e| XtaskError::InvalidConfig {
        message: format!("Failed to open '{}': {e}", path.display()),
    })?;
    let mut handle = file.take(max_bytes);
    let mut content = String::new();
    handle
        .read_to_string(&mut content)
        .map_err(|e| XtaskError::InvalidConfig {
            message: format!("Failed to read '{}': {e}", path.display()),
        })?;
    Ok(content)
}

impl AgentAdaptersManifest {
    /// Loads and parses the manifest from `MANIFEST_PATH`.
    ///
    /// # Errors
    /// Returns `XtaskError::InvalidConfig` when the manifest is missing or malformed.
    pub fn load() -> Result<Self, XtaskError> {
        Self::load_from_path(MANIFEST_PATH)
    }

    /// Loads and parses the manifest from an explicit path.
    ///
    /// # Errors
    /// Returns `XtaskError::InvalidConfig` when the file is missing or malformed.
    pub fn load_from_path(path: &str) -> Result<Self, XtaskError> {
        let content = fs::read_to_string(path).map_err(|e| XtaskError::InvalidConfig {
            message: format!("Failed to read adapter manifest '{path}': {e}"),
        })?;
        Self::from_toml(&content)
    }

    /// Parses manifest TOML content.
    ///
    /// # Errors
    /// Returns `XtaskError::InvalidConfig` when the TOML is malformed.
    pub fn from_toml(content: &str) -> Result<Self, XtaskError> {
        toml::from_str(content).map_err(|e| XtaskError::InvalidConfig {
            message: format!("Invalid adapter manifest TOML: {e}"),
        })
    }

    /// Runs the full validation pass against the repository.
    ///
    /// `repo_root` is the directory from which relative paths (entrypoints, skills, context
    /// files) are resolved. Pass the manifest's parent directory for CWD-independent behaviour.
    ///
    /// # Errors
    /// Returns `XtaskError::InvalidConfig` when the manifest itself cannot be loaded.
    pub fn validate(&self, repo_root: &Path) -> Result<ValidationResult, XtaskError> {
        let mut result = ValidationResult {
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        let canonical_content = self.validate_contract(repo_root, &mut result);
        self.validate_adapters(repo_root, canonical_content.as_ref(), &mut result);

        Ok(result)
    }

    /// Validates the contract section. Returns the canonical file content for reuse.
    fn validate_contract(&self, repo_root: &Path, result: &mut ValidationResult) -> Option<String> {
        let canonical_path = repo_root.join(&self.contract.canonical_instructions);

        // Check canonical instructions file exists and read once.
        let canonical_content = if let Ok(content) = fs::read_to_string(&canonical_path) {
            Some(content)
        } else {
            result.errors.push(ValidationFinding {
                severity: "error".to_string(),
                source: "contract".to_string(),
                message: format!(
                    "Canonical instructions file '{}' not found",
                    self.contract.canonical_instructions
                ),
            });
            None
        };

        // Check AGENTS.md line count (reusing the content already read).
        if let Some(content) = &canonical_content {
            if self.validation.max_agents_md_lines > 0 {
                let line_count = content.lines().count();
                if line_count > self.validation.max_agents_md_lines {
                    result.errors.push(ValidationFinding {
                        severity: "error".to_string(),
                        source: "contract".to_string(),
                        message: format!(
                            "{} has {line_count} lines (max {})",
                            self.contract.canonical_instructions,
                            self.validation.max_agents_md_lines
                        ),
                    });
                }
            }
        }

        // Check skills directory exists.
        let skills_path = repo_root.join(&self.contract.skills_directory);
        if !skills_path.exists() {
            result.errors.push(ValidationFinding {
                severity: "error".to_string(),
                source: "contract".to_string(),
                message: format!(
                    "Skills directory '{}' not found",
                    self.contract.skills_directory
                ),
            });
        }

        // Check context files.
        if self.validation.verify_local_links {
            for ctx_file in &self.contract.context_files {
                let ctx_path = repo_root.join(ctx_file);
                if !ctx_path.exists() {
                    result.warnings.push(ValidationFinding {
                        severity: "warning".to_string(),
                        source: "contract".to_string(),
                        message: format!("Context file '{ctx_file}' not found"),
                    });
                }
            }
        }

        canonical_content
    }

    fn validate_adapters(
        &self,
        repo_root: &Path,
        canonical_content: Option<&String>,
        result: &mut ValidationResult,
    ) {
        for adapter in &self.adapters {
            self.validate_single_adapter(repo_root, adapter, canonical_content, result);
        }
    }

    fn validate_single_adapter(
        &self,
        repo_root: &Path,
        adapter: &AdapterConfig,
        canonical_content: Option<&String>,
        result: &mut ValidationResult,
    ) {
        Self::validate_adapter_id(adapter, result);
        self.validate_adapter_entrypoint(repo_root, adapter, canonical_content, result);
    }

    fn validate_adapter_id(adapter: &AdapterConfig, result: &mut ValidationResult) {
        if adapter.id.is_empty()
            || !adapter
                .id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            result.errors.push(ValidationFinding {
                severity: "error".to_string(),
                source: adapter.id.clone(),
                message: "Adapter id must match `^[a-z][a-z0-9-]*$`".to_string(),
            });
        }
    }

    fn validate_adapter_entrypoint(
        &self,
        repo_root: &Path,
        adapter: &AdapterConfig,
        canonical_content: Option<&String>,
        result: &mut ValidationResult,
    ) {
        // Resolve entrypoint path: try repo-relative first, then relative to adapter root.
        let entrypoint_abs = repo_root.join(&adapter.entrypoint);
        let root_relative_abs = repo_root.join(&adapter.root).join(&adapter.entrypoint);

        let resolved_path = if entrypoint_abs.exists() {
            entrypoint_abs
        } else if root_relative_abs.exists() {
            root_relative_abs
        } else {
            result.errors.push(ValidationFinding {
                severity: "error".to_string(),
                source: adapter.id.clone(),
                message: format!(
                    "Entrypoint '{}' not found (checked '{}' and '{}/{}')",
                    adapter.entrypoint, adapter.entrypoint, adapter.root, adapter.entrypoint
                ),
            });
            return;
        };

        // Read entrypoint content with bounded I/O.
        let content = match read_bounded(&resolved_path, MAX_ENTRYPOINT_BYTES) {
            Ok(c) => c,
            Err(e) => {
                result.errors.push(ValidationFinding {
                    severity: "error".to_string(),
                    source: adapter.id.clone(),
                    message: format!(
                        "Failed to read entrypoint '{}': {e}",
                        resolved_path.display()
                    ),
                });
                return;
            }
        };

        // Check canonical reference using the adapter's declared canonical_reference.
        if self.validation.require_canonical_reference {
            let marker = format!("@{}", adapter.canonical_reference);
            if !content.contains(&marker) {
                result.errors.push(ValidationFinding {
                    severity: "error".to_string(),
                    source: adapter.id.clone(),
                    message: format!(
                        "Entrypoint '{}' does not contain '{marker}'",
                        adapter.entrypoint
                    ),
                });
            }
        }

        // Policy duplication: flag adapters that contain section headers from the canonical file.
        if self.validation.reject_policy_duplication {
            if let Some(canonical) = canonical_content {
                let canonical_headers: Vec<&str> =
                    canonical.lines().filter(|l| l.starts_with("## ")).collect();
                let adapter_body = strip_reference_header(&content);
                let duplicated: Vec<&str> = canonical_headers
                    .iter()
                    .filter(|h| adapter_body.contains(*h))
                    .copied()
                    .collect();
                if !duplicated.is_empty() {
                    result.warnings.push(ValidationFinding {
                        severity: "warning".to_string(),
                        source: adapter.id.clone(),
                        message: format!(
                            "Adapter may duplicate canonical sections: {}",
                            duplicated.join(", ")
                        ),
                    });
                }
            }
        }

        // Adapter scope: warn if adapter body is suspiciously long for a "thin delta".
        if self.validation.enforce_adapter_scope {
            let body = strip_reference_header(&content);
            let body_lines = body.lines().count();
            if body_lines > MAX_ADAPTER_BODY_LINES {
                result.warnings.push(ValidationFinding {
                    severity: "warning".to_string(),
                    source: adapter.id.clone(),
                    message: format!(
                        "Adapter body has {body_lines} lines (max {MAX_ADAPTER_BODY_LINES}). \
                         Thin adapters should contain only tool-specific differences."
                    ),
                });
            }
        }

        // Check root directory exists.
        let root_path = repo_root.join(&adapter.root);
        if !adapter.root.is_empty() && !root_path.exists() {
            result.warnings.push(ValidationFinding {
                severity: "warning".to_string(),
                source: adapter.id.clone(),
                message: format!("Root directory '{}' not found", adapter.root),
            });
        }
    }

    /// Generates a Markdown inventory of all registered adapters.
    #[must_use]
    pub fn inventory_markdown(&self) -> String {
        use std::fmt::Write as _;
        let mut md = String::new();
        let _ = writeln!(md, "# Agent Adapter Inventory");
        let _ = writeln!(md);
        let _ = writeln!(
            md,
            "**Canonical contract:** {}",
            self.contract.canonical_instructions
        );
        let _ = writeln!(
            md,
            "**Skills directory:** {}",
            self.contract.skills_directory
        );
        let _ = writeln!(md);
        let _ = writeln!(md, "| ID | Root | Entrypoint | Role |");
        let _ = writeln!(md, "|---|---|---|---|");
        for adapter in &self.adapters {
            let _ = writeln!(
                md,
                "| {} | {} | {} | {} |",
                adapter.id, adapter.root, adapter.entrypoint, adapter.role
            );
        }
        let _ = writeln!(md);
        let _ = writeln!(md, "## Validation Rules");
        let _ = writeln!(
            md,
            "- Require canonical reference: {}",
            self.validation.require_canonical_reference
        );
        let _ = writeln!(
            md,
            "- Reject policy duplication: {}",
            self.validation.reject_policy_duplication
        );
        let _ = writeln!(
            md,
            "- Verify local links: {}",
            self.validation.verify_local_links
        );
        let _ = writeln!(
            md,
            "- Enforce adapter scope: {}",
            self.validation.enforce_adapter_scope
        );
        let _ = writeln!(
            md,
            "- Max AGENTS.md lines: {}",
            self.validation.max_agents_md_lines
        );
        md
    }
}

/// Strips the leading `@AGENTS.md` reference line and surrounding blank lines from adapter
/// content, returning only the tool-specific body.
fn strip_reference_header(content: &str) -> String {
    content
        .lines()
        .skip_while(|l| l.trim().is_empty() || l.contains("@AGENTS.md") || l.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]
    use super::*;

    fn valid_manifest_toml() -> &'static str {
        r#"
[contract]
canonical_instructions = "AGENTS.md"
skills_directory = ".agents/skills"
context_files = ["llms.txt"]

[validation]
require_canonical_reference = true
reject_policy_duplication = true
verify_local_links = true
enforce_adapter_scope = true
max_agents_md_lines = 200

[[adapters]]
id = "claude"
root = ".claude"
entrypoint = "CLAUDE.md"
role = "tool-delta"
canonical_reference = "AGENTS.md"
"#
    }

    #[test]
    fn test_parse_valid_manifest() {
        let manifest = AgentAdaptersManifest::from_toml(valid_manifest_toml()).unwrap();
        assert_eq!(manifest.contract.canonical_instructions, "AGENTS.md");
        assert_eq!(manifest.adapters.len(), 1);
        assert_eq!(manifest.adapters[0].id, "claude");
        assert!(manifest.validation.require_canonical_reference);
    }

    #[test]
    fn test_parse_rejects_unknown_fields() {
        let toml = r#"
[contract]
canonical_instructions = "AGENTS.md"
skills_directory = ".agents/skills"
context_files = []
stray_field = true

[validation]
require_canonical_reference = true
reject_policy_duplication = true
verify_local_links = true
enforce_adapter_scope = true

[[adapters]]
id = "test"
root = ".test"
entrypoint = "TEST.md"
role = "tool-delta"
canonical_reference = "AGENTS.md"
"#;
        assert!(AgentAdaptersManifest::from_toml(toml).is_err());
    }

    #[test]
    fn test_parse_requires_adapters() {
        let toml = r#"
[contract]
canonical_instructions = "AGENTS.md"
skills_directory = ".agents/skills"
context_files = []

[validation]
require_canonical_reference = true
reject_policy_duplication = true
verify_local_links = true
enforce_adapter_scope = true
"#;
        assert!(AgentAdaptersManifest::from_toml(toml).is_err());
    }

    #[test]
    fn test_inventory_markdown() {
        let manifest = AgentAdaptersManifest::from_toml(valid_manifest_toml()).unwrap();
        let md = manifest.inventory_markdown();
        assert!(md.contains("Agent Adapter Inventory"));
        assert!(md.contains("claude"));
        assert!(md.contains("tool-delta"));
        assert!(md.contains("AGENTS.md"));
    }

    #[test]
    fn test_validation_finds_missing_entrypoint() {
        let toml = r#"
[contract]
canonical_instructions = "AGENTS.md"
skills_directory = ".agents/skills"
context_files = []

[validation]
require_canonical_reference = true
reject_policy_duplication = true
verify_local_links = false
enforce_adapter_scope = true
max_agents_md_lines = 200

[[adapters]]
id = "ghost"
root = ".ghost"
entrypoint = "GHOST_DOES_NOT_EXIST.md"
role = "tool-delta"
canonical_reference = "AGENTS.md"
"#;
        let manifest = AgentAdaptersManifest::from_toml(toml).unwrap();
        let result = manifest.validate(Path::new(".")).unwrap();
        assert!(!result.is_ok());
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.message.contains("GHOST_DOES_NOT_EXIST.md"))
        );
    }

    #[test]
    fn test_validation_finds_invalid_adapter_id() {
        let toml = r#"
[contract]
canonical_instructions = "AGENTS.md"
skills_directory = ".agents/skills"
context_files = []

[validation]
require_canonical_reference = false
reject_policy_duplication = true
verify_local_links = false
enforce_adapter_scope = true
max_agents_md_lines = 200

[[adapters]]
id = "Bad_ID"
root = ".bad"
entrypoint = "BAD.md"
role = "tool-delta"
canonical_reference = "AGENTS.md"
"#;
        let manifest = AgentAdaptersManifest::from_toml(toml).unwrap();
        let result = manifest.validate(Path::new(".")).unwrap();
        assert!(!result.is_ok());
        assert!(result.errors.iter().any(|e| e.source == "Bad_ID"));
    }

    #[test]
    fn test_validation_result_print_report() {
        let result = ValidationResult {
            errors: vec![ValidationFinding {
                severity: "error".to_string(),
                source: "test".to_string(),
                message: "test error".to_string(),
            }],
            warnings: vec![ValidationFinding {
                severity: "warning".to_string(),
                source: "test".to_string(),
                message: "test warning".to_string(),
            }],
        };
        assert!(!result.is_ok());
        // Should not panic.
        result.print_report();
    }

    #[test]
    fn test_uses_canonical_reference_field() {
        // Adapter declares canonical_reference = "FOO.md" — validator should check for @FOO.md
        let dir = tempfile::tempdir().unwrap();
        // Create a file that contains @AGENTS.md but not @FOO.md.
        fs::write(dir.path().join("TEST.md"), "# Test\n@AGENTS.md\n").unwrap();

        let toml = r#"
[contract]
canonical_instructions = "AGENTS.md"
skills_directory = ".agents/skills"
context_files = []

[validation]
require_canonical_reference = true
reject_policy_duplication = false
verify_local_links = false
enforce_adapter_scope = false
max_agents_md_lines = 200

[[adapters]]
id = "test"
root = ""
entrypoint = "TEST.md"
role = "tool-delta"
canonical_reference = "FOO.md"
"#;
        let manifest = AgentAdaptersManifest::from_toml(toml).unwrap();
        let result = manifest.validate(dir.path()).unwrap();
        // TEST.md contains @AGENTS.md but not @FOO.md — should error.
        assert!(!result.is_ok());
        assert!(result.errors.iter().any(|e| e.message.contains("@FOO.md")));
    }

    #[test]
    fn test_read_bounded_rejects_large_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("huge.md");
        // Write 2 MiB of content.
        let content = "x".repeat(2_097_152);
        fs::write(&path, &content).unwrap();
        let result = read_bounded(&path, 1024);
        // Should succeed but be truncated (read_to_string on a taken reader).
        let text = result.unwrap();
        assert!(text.len() <= 1024);
    }
}
