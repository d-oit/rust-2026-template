//! Validated project blueprints (issue #286).
//!
//! A `TemplateProfile` is a declarative TOML blueprint that decides which crates,
//! directories, workflows, CI tier, and post-init checklist a generated project keeps.
//! Profiles live in `config/template-profiles/` and are validated before use.

use crate::config::XtaskError;
use crate::path_rules::{
    validate_exclude_path, validate_exclude_workflow, validate_include_crate,
    validate_package_name, validate_profile_id_str,
};
use serde::Deserialize;
use std::fs;
use std::path::Path;

/// Directory holding the shipped profile blueprints, relative to the repository root.
pub const PROFILES_DIR: &str = "config/template-profiles";

/// The ids of the shipped profiles, in display order.
pub const SHIPPED_PROFILES: &[&str] = &[
    "minimal",
    "library",
    "cli",
    "service",
    "workspace",
    "ai-agent",
];

/// Profile metadata.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileMetadata {
    /// Machine id (`^[a-z][a-z0-9-]*$`), e.g. "library". Used as `--profile <id>`.
    pub id: String,
    /// Human title, e.g. "Rust Library".
    pub display_name: String,
    /// One-line description.
    pub description: String,
}

/// Workspace-shaping decisions.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileWorkspace {
    /// Crates under `crates/` to KEEP (any other crate is removed).
    pub include_crates: Vec<String>,
    /// Non-crate paths to remove (`benchmarks`, `fuzz`, `.template`, …).
    #[serde(default)]
    pub exclude_paths: Vec<String>,
    /// Workflow files under `.github/workflows/` to remove.
    #[serde(default)]
    pub exclude_workflows: Vec<String>,
}

/// CI defaults applied by this profile.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileCi {
    /// Default verification tier to write into `config/xtask.json`.
    pub default_tier: String,
}

/// Post-init guidance that cannot travel through a GitHub template.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostInit {
    /// Ordered checklist items for the generated project's README/ADR.
    pub checklist: Vec<String>,
}

/// How the generated project handles `Cargo.lock`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LockfilePolicy {
    /// Commit `Cargo.lock` (un-ignore it) — reproducible builds for binaries/services.
    Committed,
    /// Keep `Cargo.lock` ignored — reasonable for pure library workspaces.
    Ignored,
}

/// Explicit post-init policy decisions (replacing open checklist questions).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfilePolicy {
    /// Whether the generated project commits `Cargo.lock`.
    #[serde(default = "default_lockfile_policy")]
    pub lockfile: LockfilePolicy,
    /// Crate names the adopting project intends to publish (empty: publish nothing).
    #[serde(default)]
    pub publish_packages: Vec<String>,
}

const fn default_lockfile_policy() -> LockfilePolicy {
    LockfilePolicy::Committed
}

impl Default for ProfilePolicy {
    fn default() -> Self {
        Self {
            lockfile: default_lockfile_policy(),
            publish_packages: Vec::new(),
        }
    }
}

/// A validated profile blueprint.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateProfile {
    /// Profile metadata.
    pub metadata: ProfileMetadata,
    /// Workspace-shaping decisions.
    pub workspace: ProfileWorkspace,
    /// CI defaults.
    pub ci: ProfileCi,
    /// Explicit lockfile/publication policy.
    #[serde(default)]
    pub policy: ProfilePolicy,
    /// Post-init checklist.
    pub post_init: PostInit,
}

impl TemplateProfile {
    /// Parses and structurally validates profile TOML.
    ///
    /// # Errors
    /// Returns `XtaskError::InvalidConfig` when the TOML is malformed or violates the schema.
    pub fn from_toml(content: &str) -> Result<Self, XtaskError> {
        let profile: Self = toml::from_str(content).map_err(|e| XtaskError::InvalidConfig {
            message: format!("Invalid profile TOML: {e}"),
        })?;
        profile.validate()
    }

    /// Loads a profile by id from `PROFILES_DIR`.
    ///
    /// Only shipped profile ids are accepted: the id is validated as a safe
    /// identifier *before* any path is constructed, preventing traversal
    /// (e.g. `../evil`) from reaching the filesystem.
    ///
    /// # Errors
    /// Returns `XtaskError::InvalidConfig` when the profile is unknown or invalid.
    pub fn load(id: &str) -> Result<Self, XtaskError> {
        if let Err(reason) = validate_profile_id_str(id) {
            return Err(XtaskError::InvalidConfig {
                message: format!(
                    "invalid profile id: {reason}; expected one of {SHIPPED_PROFILES:?}"
                ),
            });
        }
        if !SHIPPED_PROFILES.contains(&id) {
            return Err(XtaskError::InvalidConfig {
                message: format!(
                    "invalid profile id: '{id}' is not a shipped profile; expected one of {SHIPPED_PROFILES:?}"
                ),
            });
        }
        let path = format!("{PROFILES_DIR}/{id}.toml");
        Self::load_from_path(&path).map_err(|e| XtaskError::InvalidConfig {
            message: format!("Failed to load profile '{id}': {e}"),
        })
    }

    /// Loads a profile from an explicit path (used by `validate-profile`).
    ///
    /// # Errors
    /// Returns `XtaskError::InvalidConfig` when the file is missing or invalid.
    pub fn load_from_path(path: &str) -> Result<Self, XtaskError> {
        let content = fs::read_to_string(path).map_err(|e| XtaskError::InvalidConfig {
            message: format!("Failed to read profile '{path}': {e}"),
        })?;
        Self::from_toml(&content)
    }

    /// Validates the profile against the structural rules of `schema/template-profile.schema.json`.
    ///
    /// Every path-like field is checked with `Path::components` so no entry may
    /// escape its permitted root (`..`, absolute paths, platform prefixes,
    /// embedded backslashes, or control characters).
    ///
    /// # Errors
    /// Returns `XtaskError::InvalidConfig` on the first violated rule.
    pub fn validate(&self) -> Result<Self, XtaskError> {
        validate_profile_id_str(&self.metadata.id)
            .map_err(|reason| self.invalid(&format!("metadata.id {reason}")))?;
        if self.metadata.display_name.trim().is_empty() {
            return Err(self.invalid("metadata.display_name must not be empty"));
        }
        if self.workspace.include_crates.is_empty() {
            return Err(self.invalid("workspace.include_crates must contain at least one crate"));
        }
        for include in &self.workspace.include_crates {
            validate_include_crate(include).map_err(|reason| {
                self.invalid(&format!("workspace.include_crates entry {reason}"))
            })?;
        }
        for excluded in &self.workspace.exclude_paths {
            validate_exclude_path(excluded).map_err(|reason| {
                self.invalid(&format!("workspace.exclude_paths entry {reason}"))
            })?;
        }
        for wf in &self.workspace.exclude_workflows {
            validate_exclude_workflow(wf).map_err(|reason| {
                self.invalid(&format!("workspace.exclude_workflows entry {reason}"))
            })?;
        }
        if self.ci.default_tier.trim().is_empty() {
            return Err(self.invalid("ci.default_tier must not be empty"));
        }
        for package in &self.policy.publish_packages {
            validate_package_name(package).map_err(|reason| {
                self.invalid(&format!("policy.publish_packages entry {reason}"))
            })?;
        }
        Ok(self.clone())
    }

    /// Computes the crates under `crates/` that this profile removes (those not included).
    #[must_use]
    pub fn removed_crates(&self, existing_crates: &[String]) -> Vec<String> {
        existing_crates
            .iter()
            .filter(|crate_name| {
                let include_path = format!("crates/{crate_name}");
                !self
                    .workspace
                    .include_crates
                    .iter()
                    .any(|i| i == &include_path)
            })
            .cloned()
            .collect()
    }

    /// Prints a structured inspection summary.
    pub fn inspect(&self) {
        println!(
            "Profile: {} ({})",
            self.metadata.id, self.metadata.display_name
        );
        println!("  {}", self.metadata.description);
        println!("Workspace includes:");
        for crate_path in &self.workspace.include_crates {
            println!("  - {crate_path}");
        }
        if !self.workspace.exclude_paths.is_empty() {
            println!("Excludes paths:");
            for p in &self.workspace.exclude_paths {
                println!("  - {p}");
            }
        }
        if !self.workspace.exclude_workflows.is_empty() {
            println!("Excludes workflows:");
            for wf in &self.workspace.exclude_workflows {
                println!("  - {wf}");
            }
        }
        println!("CI default tier: {}", self.ci.default_tier);
        println!(
            "Policy: lockfile={:?}, publish_packages={:?}",
            self.policy.lockfile, self.policy.publish_packages
        );
        println!("Post-init checklist:");
        for item in &self.post_init.checklist {
            println!("  - [ ] {item}");
        }
    }

    fn invalid(&self, message: &str) -> XtaskError {
        XtaskError::InvalidConfig {
            message: format!("Profile '{}' invalid: {message}", self.metadata.id),
        }
    }
}

/// Lists supplied profiles known to exist on disk (for docs/tests).
#[must_use]
pub fn shipped_profile_paths(root: &Path) -> Vec<std::path::PathBuf> {
    SHIPPED_PROFILES
        .iter()
        .map(|id| root.join(format!("{PROFILES_DIR}/{id}.toml")))
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]
    use super::*;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
    }

    fn profile_path(id: &str) -> String {
        repo_root()
            .join(format!("{PROFILES_DIR}/{id}.toml"))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn test_all_shipped_profiles_parse_and_validate() {
        for id in SHIPPED_PROFILES {
            let profile = TemplateProfile::load_from_path(&profile_path(id))
                .unwrap_or_else(|e| panic!("profile {id} must load: {e}"));
            assert_eq!(profile.metadata.id, *id);
            assert!(!profile.workspace.include_crates.is_empty());
            assert!(!profile.ci.default_tier.is_empty());
            assert!(!profile.post_init.checklist.is_empty());
        }
    }

    #[test]
    fn test_unknown_profile_errors() {
        let err = TemplateProfile::load("no-such-profile").unwrap_err();
        assert!(
            err.to_string().contains("invalid profile id"),
            "unknown profile must be rejected as invalid id, got: {err}"
        );
    }

    #[test]
    fn test_validate_rejects_empty_include_crates() {
        let toml = r#"
[metadata]
id = "broken"
display_name = "Broken"
description = "no includes"
[workspace]
include_crates = []
[ci]
default_tier = "pull-request"
[post_init]
checklist = ["x"]
"#;
        let err = TemplateProfile::from_toml(toml).unwrap_err();
        assert!(err.to_string().contains("include_crates"));
    }

    #[test]
    fn test_validate_rejects_unknown_field() {
        let toml = r#"
[metadata]
id = "broken"
display_name = "Broken"
description = "extra field"
[workspace]
include_crates = ["crates/xtask"]
stray = true
[ci]
default_tier = "pull-request"
[post_init]
checklist = ["x"]
"#;
        assert!(TemplateProfile::from_toml(toml).is_err());
    }

    #[test]
    fn test_removed_crates_plans() {
        let toml = r#"
[metadata]
id = "t"
display_name = "T"
description = "d"
[workspace]
include_crates = ["crates/xtask"]
exclude_paths = ["benchmarks"]
exclude_workflows = ["fuzz.yml"]
[ci]
default_tier = "pull-request"
[post_init]
checklist = ["x"]
"#;
        let profile = TemplateProfile::from_toml(toml).unwrap();
        let existing = vec!["xtask".to_string(), "sample-app".to_string()];
        assert_eq!(profile.removed_crates(&existing), vec!["sample-app"]);
        assert_eq!(profile.workspace.exclude_paths, vec!["benchmarks"]);
        assert_eq!(profile.workspace.exclude_workflows, vec!["fuzz.yml"]);
    }
}

#[cfg(test)]
#[path = "template_profile_test.rs"]
mod template_profile_test;
