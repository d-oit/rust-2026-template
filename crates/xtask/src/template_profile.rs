//! Validated project blueprints (issue #286).
//!
//! A `TemplateProfile` is a declarative TOML blueprint that decides which crates,
//! directories, workflows, CI tier, and post-init checklist a generated project keeps.
//! Profiles live in `config/template-profiles/` and are validated before use.

use crate::config::XtaskError;
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
    /// # Errors
    /// Returns `XtaskError::InvalidConfig` when the profile is unknown or invalid.
    pub fn load(id: &str) -> Result<Self, XtaskError> {
        let path = format!("{PROFILES_DIR}/{id}.toml");
        Self::load_from_path(&path).map_err(|e| XtaskError::InvalidConfig {
            message: format!("Unknown profile '{id}' (expected one of {SHIPPED_PROFILES:?}): {e}"),
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
    /// # Errors
    /// Returns `XtaskError::InvalidConfig` on the first violated rule.
    pub fn validate(&self) -> Result<Self, XtaskError> {
        if self.metadata.id.is_empty()
            || !self
                .metadata
                .id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(self.invalid("metadata.id must be `^[a-z][a-z0-9-]*$`"));
        }
        if self.metadata.display_name.trim().is_empty() {
            return Err(self.invalid("metadata.display_name must not be empty"));
        }
        if self.workspace.include_crates.is_empty() {
            return Err(self.invalid("workspace.include_crates must contain at least one crate"));
        }
        for include in &self.workspace.include_crates {
            if !include.starts_with("crates/") {
                return Err(self.invalid(&format!(
                    "workspace.include_crates entry must start with 'crates/', got '{include}'"
                )));
            }
        }
        for wf in &self.workspace.exclude_workflows {
            if !wf.ends_with(".yml") && !wf.ends_with(".yaml") {
                return Err(self.invalid(&format!(
                    "workspace.exclude_workflows entry must end with .yml or .yaml, got '{wf}'"
                )));
            }
        }
        if self.ci.default_tier.trim().is_empty() {
            return Err(self.invalid("ci.default_tier must not be empty"));
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
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        if manifest_dir.join("../../config/template-profiles").exists() {
            manifest_dir.join("..").join("..")
        } else if let Ok(cwd) = std::env::current_dir() {
            let mut curr = cwd;
            loop {
                if curr.join("config/template-profiles").exists() {
                    return curr;
                }
                if !curr.pop() {
                    break;
                }
            }
            manifest_dir.join("..").join("..")
        } else {
            manifest_dir.join("..").join("..")
        }
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
        assert!(err.to_string().contains("Unknown profile"));
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
