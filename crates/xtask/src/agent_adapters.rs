//! Agent adapter manifest validation (issue #287).
//!
//! Validates that tool-specific adapters remain thin wrappers around the canonical
//! `AGENTS.md` contract. The manifest at `.agents/agent-adapters.toml` declares the
//! expected adapter topology; this module enforces it programmatically.

use crate::config::XtaskError;
use serde::Deserialize;
use std::fs;
use std::io::Read as _;
use std::path::Path;

/// Path to the adapter manifest, relative to the repository root.
pub const MANIFEST_PATH: &str = ".agents/agent-adapters.toml";
const MAX_ENTRYPOINT_BYTES: u64 = 1_048_576;
const MAX_ADAPTER_BODY_LINES: usize = 40;

/// Canonical contract declaration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractConfig {
    /// Path to the canonical instructions file.
    pub canonical_instructions: String,
    /// Path to the shared skills directory.
    pub skills_directory: String,
    /// Context files shipped for agent consumption.
    pub context_files: Vec<String>,
}

/// Validation rules for adapter enforcement.
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
    /// Root directory for this adapter's files.
    pub root: String,
    /// Entrypoint file relative to the repository root.
    pub entrypoint: String,
    /// Adapter role (currently only "tool-delta").
    pub role: String,
    /// The canonical file this adapter must reference.
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

    /// Validates using CWD-derived repo root.
    ///
    /// # Errors
    /// Returns `XtaskError` when the manifest cannot be loaded or validated.
    pub fn validate_from_cwd(&self) -> Result<ValidationResult, XtaskError> {
        let repo_root = Self::repo_root()?;
        self.validate(&repo_root)
    }

    /// Runs the full validation pass against the repository.
    ///
    /// # Errors
    /// Returns `XtaskError` when validation infrastructure fails.
    pub fn validate(&self, repo_root: &Path) -> Result<ValidationResult, XtaskError> {
        let mut result = ValidationResult {
            errors: Vec::new(),
            warnings: Vec::new(),
        };
        let canonical = self.validate_contract(repo_root, &mut result);
        self.validate_adapters(repo_root, canonical.as_ref(), &mut result);
        Ok(result)
    }

    /// Prints a plain-text inventory of all registered adapters.
    pub fn print_inventory_plain(&self) {
        println!("Adapters:");
        for a in &self.adapters {
            println!("  {} -> {} ({})", a.id, a.entrypoint, a.role);
        }
    }

    /// Verifies that declared context files exist on disk.
    ///
    /// # Errors
    /// Returns `XtaskError::CommandFailure` when any context file is missing.
    pub fn check_context(&self) -> Result<(), XtaskError> {
        let mut ok = true;
        for f in &self.contract.context_files {
            if Path::new(f).exists() {
                println!("  ✅ {f}");
            } else {
                println!("  ❌ {f} — missing");
                ok = false;
            }
        }
        if ok {
            Ok(())
        } else {
            Err(XtaskError::CommandFailure {
                command: "agents check-context".into(),
                exit_code: Some(1),
            })
        }
    }

    fn repo_root() -> Result<std::path::PathBuf, XtaskError> {
        let p = Path::new(MANIFEST_PATH);
        Ok(if p.exists() {
            p.canonicalize()
                .map_err(|e| XtaskError::InvalidConfig {
                    message: format!("Failed to resolve manifest path: {e}"),
                })?
                .parent()
                .and_then(|pp| pp.parent())
                .map_or_else(|| ".".into(), Path::to_path_buf)
        } else {
            std::env::current_dir().unwrap_or_else(|_| ".".into())
        })
    }

    fn validate_contract(&self, repo_root: &Path, result: &mut ValidationResult) -> Option<String> {
        let path = repo_root.join(&self.contract.canonical_instructions);
        let content = if let Ok(c) = fs::read_to_string(&path) {
            Some(c)
        } else {
            result.errors.push(err(
                "contract",
                &format!(
                    "Canonical instructions file '{}' not found",
                    self.contract.canonical_instructions
                ),
            ));
            None
        };
        if let Some(c) = &content {
            let n = c.lines().count();
            if self.validation.max_agents_md_lines > 0 && n > self.validation.max_agents_md_lines {
                result.errors.push(err(
                    "contract",
                    &format!(
                        "{} has {n} lines (max {})",
                        self.contract.canonical_instructions, self.validation.max_agents_md_lines
                    ),
                ));
            }
        }
        if !repo_root.join(&self.contract.skills_directory).exists() {
            result.errors.push(err(
                "contract",
                &format!(
                    "Skills directory '{}' not found",
                    self.contract.skills_directory
                ),
            ));
        }
        if self.validation.verify_local_links {
            for f in &self.contract.context_files {
                if !repo_root.join(f).exists() {
                    result
                        .warnings
                        .push(warn("contract", &format!("Context file '{f}' not found")));
                }
            }
        }
        content
    }

    fn validate_adapters(
        &self,
        repo_root: &Path,
        canonical: Option<&String>,
        result: &mut ValidationResult,
    ) {
        for adapter in &self.adapters {
            self.validate_single(repo_root, adapter, canonical, result);
        }
    }

    fn validate_single(
        &self,
        repo_root: &Path,
        a: &AdapterConfig,
        canonical: Option<&String>,
        result: &mut ValidationResult,
    ) {
        // ID format.
        if a.id.is_empty()
            || !a
                .id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            result
                .errors
                .push(err(&a.id, "Adapter id must match `^[a-z][a-z0-9-]*$`"));
        }
        // Entrypoint resolution.
        let ep = repo_root.join(&a.entrypoint);
        let root_ep = repo_root.join(&a.root).join(&a.entrypoint);
        let resolved = if ep.exists() {
            ep
        } else if root_ep.exists() {
            root_ep
        } else {
            result.errors.push(err(
                &a.id,
                &format!(
                    "Entrypoint '{}' not found (checked '{}' and '{}/{}')",
                    a.entrypoint, a.entrypoint, a.root, a.entrypoint
                ),
            ));
            return;
        };
        // Read with bounds.
        let content = match read_bounded(&resolved, MAX_ENTRYPOINT_BYTES) {
            Ok(c) => c,
            Err(e) => {
                result.errors.push(err(
                    &a.id,
                    &format!("Failed to read entrypoint '{}': {e}", resolved.display()),
                ));
                return;
            }
        };
        // Canonical reference check.
        if self.validation.require_canonical_reference {
            let marker = format!("@{}", a.canonical_reference);
            if !content.contains(&marker) {
                result.errors.push(err(
                    &a.id,
                    &format!("Entrypoint '{}' does not contain '{marker}'", a.entrypoint),
                ));
            }
        }
        // Policy duplication.
        if self.validation.reject_policy_duplication {
            if let Some(c) = canonical {
                let headers: Vec<&str> = c.lines().filter(|l| l.starts_with("## ")).collect();
                let body = strip_ref_header(&content);
                let dup: Vec<&str> = headers
                    .iter()
                    .filter(|h| body.contains(*h))
                    .copied()
                    .collect();
                if !dup.is_empty() {
                    result.warnings.push(warn(
                        &a.id,
                        &format!(
                            "Adapter may duplicate canonical sections: {}",
                            dup.join(", ")
                        ),
                    ));
                }
            }
        }
        // Scope enforcement.
        if self.validation.enforce_adapter_scope {
            let n = strip_ref_header(&content).lines().count();
            if n > MAX_ADAPTER_BODY_LINES {
                result.warnings.push(warn(
                    &a.id,
                    &format!(
                        "Adapter body has {n} lines (max {MAX_ADAPTER_BODY_LINES}). \
                     Thin adapters should contain only tool-specific differences."
                    ),
                ));
            }
        }
        // Root directory.
        if !a.root.is_empty() && !repo_root.join(&a.root).exists() {
            result.warnings.push(warn(
                &a.id,
                &format!("Root directory '{}' not found", a.root),
            ));
        }
    }

    /// Generates a Markdown inventory of all registered adapters.
    #[must_use]
    pub fn inventory_markdown(&self) -> String {
        use std::fmt::Write as _;
        let mut md = String::new();
        let _ = writeln!(md, "# Agent Adapter Inventory\n");
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
        let _ = writeln!(md, "\n| ID | Root | Entrypoint | Role |\n|---|---|---|---|");
        for a in &self.adapters {
            let _ = writeln!(
                md,
                "| {} | {} | {} | {} |",
                a.id, a.root, a.entrypoint, a.role
            );
        }
        let v = &self.validation;
        let _ = writeln!(md, "\n## Validation Rules");
        let _ = writeln!(
            md,
            "- Require canonical reference: {}",
            v.require_canonical_reference
        );
        let _ = writeln!(
            md,
            "- Reject policy duplication: {}",
            v.reject_policy_duplication
        );
        let _ = writeln!(md, "- Verify local links: {}", v.verify_local_links);
        let _ = writeln!(md, "- Enforce adapter scope: {}", v.enforce_adapter_scope);
        let _ = writeln!(md, "- Max AGENTS.md lines: {}", v.max_agents_md_lines);
        md
    }
}

fn err(source: &str, message: &str) -> ValidationFinding {
    ValidationFinding {
        severity: "error".into(),
        source: source.into(),
        message: message.into(),
    }
}
fn warn(source: &str, message: &str) -> ValidationFinding {
    ValidationFinding {
        severity: "warning".into(),
        source: source.into(),
        message: message.into(),
    }
}

fn strip_ref_header(content: &str) -> String {
    content
        .lines()
        .skip_while(|l| l.trim().is_empty() || l.contains("@AGENTS.md") || l.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
#[path = "agent_adapters_test.rs"]
mod tests;
