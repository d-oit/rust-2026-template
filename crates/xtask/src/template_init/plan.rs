//! The immutable initialization plan.
//!
//! `InitPlan::build` performs *all* reads, validation, and content preparation
//! up front. It mutates nothing: every filesystem operation the apply step will
//! perform is decided and pre-computed here, so a failure during planning (or
//! during apply) is either a no-op or a clean abort rather than partial state.

use super::validate::{self, ProjectIdentity};
use crate::config::XtaskError;
use crate::template_profile::{LockfilePolicy, TemplateProfile};
use std::path::{Path, PathBuf};
use toml_edit::{Array, DocumentMut, value};

#[cfg(test)]
#[path = "plan_test.rs"]
mod plan_test;

/// A file rewrite prepared ahead of execution (written atomically by apply).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRewrite {
    /// Absolute destination path.
    pub path: PathBuf,
    /// Fully prepared new content.
    pub content: String,
}

/// A rename of the example crate to the project's own name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateRename {
    /// Existing example-crate directory (canonicalized, contained in root).
    pub from: PathBuf,
    /// Destination directory (validated to not exist).
    pub to: PathBuf,
}

/// Everything template initialization will do, decided be/model/fore touching disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitPlan {
    /// Canonical repository root the plan is anchored to.
    pub root: PathBuf,
    /// Profile id the plan was built from.
    pub profile_id: String,
    /// Validated caller-supplied project identity.
    pub identity: ProjectIdentity,
    /// CI default tier to write into `config/xtask.json`.
    pub default_tier: String,
    /// Existing paths (canonical, contained in root) to remove.
    pub removals: Vec<PathBuf>,
    /// Example-crate rename, if applicable.
    pub rename: Option<CrateRename>,
    /// Root `Cargo.toml` rewrite (members, default-members, package identity).
    pub workspace_manifest: Option<FileRewrite>,
    /// Renamed crate manifest rewrite (written at the *new* path).
    pub crate_manifest: Option<FileRewrite>,
    /// `config/xtask.json` rewrite with the profile's default tier.
    pub ci_config: Option<FileRewrite>,
    /// `.gitignore` rewrite implementing the lockfile policy.
    pub gitignore: Option<FileRewrite>,
    /// Prose placeholder rewrites for the fixed allowlist of doc/example files.
    pub text_replacements: Vec<FileRewrite>,
    /// Whether the generated project commits `Cargo.lock`.
    pub lockfile_committed: bool,
    /// Crate names the adopter intends to publish (recorded for release tooling).
    pub publish_packages: Vec<String>,
    /// Post-init checklist to print after applying.
    pub checklist: Vec<String>,
}

/// Lines removed from `.gitignore` when the lockfile policy is `committed`.
const LOCKFILE_IGNORE_LINES: [&str; 4] = [
    "# Cargo.lock is intentionally excluded for this library/template repo.",
    "# If you adopt this template for a *binary* application, remove this line",
    "# and commit your Cargo.lock. See README.md#cargo-lock-policy for details.",
    "Cargo.lock",
];

impl InitPlan {
    /// Builds the complete plan. Reads and validates everything; mutates nothing.
    ///
    /// # Errors
    /// Returns `XtaskError::InvalidConfig` when any target is missing, escapes
    /// the repository root, or conflicts (e.g. rename destination exists).
    pub(crate) fn build(
        root: &Path,
        blueprint: &TemplateProfile,
        identity: &ProjectIdentity,
    ) -> Result<Self, XtaskError> {
        let crates_dir = root.join("crates");
        let existing = list_crate_dirs(&crates_dir)?;
        let removed_names = blueprint.removed_crates(&existing);
        // Literal workspace-member strings that must disappear from the manifest.
        let mut removed_paths: Vec<String> = removed_names
            .iter()
            .map(|name| format!("crates/{name}"))
            .collect();
        removed_paths.extend(blueprint.workspace.exclude_paths.iter().cloned());

        let mut removals = Vec::new();
        for name in &removed_names {
            push_contained(&mut removals, &crates_dir.join(name), root)?;
        }
        for rel in &blueprint.workspace.exclude_paths {
            push_contained_if_exists(&mut removals, &root.join(rel), root)?;
        }
        for wf in &blueprint.workspace.exclude_workflows {
            let path = root.join(".github").join("workflows").join(wf);
            push_contained_if_exists(&mut removals, &path, root)?;
        }

        let rename = plan_rename(root, &crates_dir, identity)?;

        let root_manifest_path = root.join("Cargo.toml");
        let root_manifest_content = read_required(&root_manifest_path)?;
        let workspace_manifest =
            edit_workspace_manifest(&root_manifest_content, &removed_paths, identity)?.map(
                |content| FileRewrite {
                    path: root_manifest_path,
                    content,
                },
            );

        let crate_manifest = match rename.as_ref() {
            Some(r) => {
                let src = r.from.join("Cargo.toml");
                let content = read_required(&src)?;
                let edited = edit_crate_manifest(&content, identity).ok_or_else(|| {
                    XtaskError::InvalidConfig {
                        message: format!("{} is not valid TOML", src.display()),
                    }
                })?;
                Some(FileRewrite {
                    path: r.to.join("Cargo.toml"),
                    content: edited,
                })
            }
            None => None,
        };

        let ci_config_path = root.join("config").join("xtask.json");
        let ci_config = if ci_config_path.exists() {
            let content = std::fs::read_to_string(&ci_config_path).map_err(|e| {
                XtaskError::InvalidConfig {
                    message: format!("failed to read config/xtask.json: {e}"),
                }
            })?;
            Some(FileRewrite {
                path: ci_config_path,
                content: edit_ci_config(&content, &blueprint.ci.default_tier)?,
            })
        } else {
            None
        };

        let gitignore_path = root.join(".gitignore");
        let gitignore =
            if blueprint.policy.lockfile == LockfilePolicy::Committed && gitignore_path.exists() {
                let content = std::fs::read_to_string(&gitignore_path).map_err(|e| {
                    XtaskError::InvalidConfig {
                        message: format!("failed to read .gitignore: {e}"),
                    }
                })?;
                edit_gitignore(&content).map(|content| FileRewrite {
                    path: gitignore_path,
                    content,
                })
            } else {
                None
            };

        let text_replacements = prose_rewrites(root, rename.as_ref(), identity);

        Ok(Self {
            root: root.to_path_buf(),
            profile_id: blueprint.metadata.id.clone(),
            identity: identity.clone(),
            default_tier: blueprint.ci.default_tier.clone(),
            removals,
            rename,
            workspace_manifest,
            crate_manifest,
            ci_config,
            gitignore,
            text_replacements,
            lockfile_committed: blueprint.policy.lockfile == LockfilePolicy::Committed,
            publish_packages: blueprint.policy.publish_packages.clone(),
            checklist: blueprint.post_init.checklist.clone(),
        })
    }

    /// Prints the plan without executing it (dry-run output).
    pub(crate) fn print_dry_run(&self) {
        println!(
            "  [DRY RUN] Would initialize project '{}' ({})",
            self.identity.name, self.identity.description
        );
        println!(
            "  [DRY RUN] Would remove {} path(s): {:?}",
            self.removals.len(),
            self.removals
                .iter()
                .map(|p| p.strip_prefix(&self.root).unwrap_or(p).to_path_buf())
                .collect::<Vec<_>>()
        );
        match &self.rename {
            Some(r) => println!(
                "  [DRY RUN] Would rename {} to {}",
                r.from.display(),
                r.to.display()
            ),
            None => println!("  [DRY RUN] No example crate to rename"),
        }
        println!(
            "  [DRY RUN] Would rewrite {} file(s) (manifests, CI tier, lockfile policy, prose)",
            usize::from(self.workspace_manifest.is_some())
                + usize::from(self.crate_manifest.is_some())
                + usize::from(self.ci_config.is_some())
                + usize::from(self.gitignore.is_some())
                + self.text_replacements.len()
        );
        println!(
            "  [DRY RUN] Would set default CI tier: {}",
            self.default_tier
        );
        println!(
            "  [DRY RUN] Policy: lockfile={}, publish_packages={:?}",
            if self.lockfile_committed {
                "committed"
            } else {
                "ignored"
            },
            self.publish_packages
        );
    }
}

fn push_contained(target: &mut Vec<PathBuf>, path: &Path, root: &Path) -> Result<(), XtaskError> {
    let canonical = validate::contained_canonical(path, root)?;
    target.push(canonical);
    Ok(())
}

fn push_contained_if_exists(
    target: &mut Vec<PathBuf>,
    path: &Path,
    root: &Path,
) -> Result<(), XtaskError> {
    if path.symlink_metadata().is_ok() {
        push_contained(target, path, root)?;
    }
    Ok(())
}

fn list_crate_dirs(crates_dir: &Path) -> Result<Vec<String>, XtaskError> {
    let entries = std::fs::read_dir(crates_dir).map_err(|e| XtaskError::InvalidConfig {
        message: format!("failed to read crates/ directory: {e}"),
    })?;
    let mut names = Vec::new();
    for entry in entries.flatten() {
        if entry.path().is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.to_string());
            }
        }
    }
    names.sort_unstable();
    Ok(names)
}

fn read_required(path: &Path) -> Result<String, XtaskError> {
    std::fs::read_to_string(path).map_err(|e| XtaskError::InvalidConfig {
        message: format!("failed to read required file {}: {e}", path.display()),
    })
}

fn plan_rename(
    root: &Path,
    crates_dir: &Path,
    identity: &ProjectIdentity,
) -> Result<Option<CrateRename>, XtaskError> {
    let example = crates_dir.join("example-crate");
    if !example.is_dir() || identity.name == "example-crate" {
        return Ok(None);
    }
    let from = validate::contained_canonical(&example, root)?;
    let to = crates_dir.join(&identity.name);
    if to.symlink_metadata().is_ok() {
        return Err(XtaskError::InvalidConfig {
            message: format!(
                "rename destination {} already exists; refusing to overwrite",
                to.display()
            ),
        });
    }
    Ok(Some(CrateRename { from, to }))
}

/// Structural root-manifest edits: drop removed members, repair
/// `default-members`, and write the validated package identity.
fn edit_workspace_manifest(
    content: &str,
    removed_paths: &[String],
    identity: &ProjectIdentity,
) -> Result<Option<String>, XtaskError> {
    let mut doc: DocumentMut = content.parse().map_err(|e| XtaskError::InvalidConfig {
        message: format!("root Cargo.toml is not valid TOML: {e}"),
    })?;
    let mut changed = false;

    if let Some(members) = doc["workspace"]["members"].as_array_mut() {
        let before = members.len();
        members.retain(|item| {
            let entry = item.as_str().unwrap_or_default();
            !removed_paths.iter().any(|removed| removed == entry)
        });
        changed |= members.len() != before;
    }

    if let Some(default_members) = doc["workspace"]["default-members"].as_array_mut() {
        let before = default_members.len();
        default_members.retain(|item| {
            let entry = item.as_str().unwrap_or_default();
            !removed_paths.iter().any(|removed| removed == entry)
        });
        if default_members.is_empty() {
            if let Some(workspace) = doc["workspace"].as_table_mut() {
                workspace.remove("default-members");
            }
            changed = true;
        } else {
            changed |= default_members.len() != before;
        }
    }

    if let Some(package) = doc["workspace"]["package"].as_table_mut() {
        let mut authors = Array::new();
        authors.push(identity.author.as_str());
        package["authors"] = value(authors);
        package["repository"] = value(format!("https://github.com/{}", identity.repo));
        package["homepage"] = value(format!("https://github.com/{}", identity.repo));
        changed = true;
    }

    if changed {
        Ok(Some(doc.to_string()))
    } else {
        Ok(None)
    }
}

/// Structural renamed-crate manifest edits: package name and description, then
/// a prose pass over any remaining `example-crate` identifiers (e.g. lib name).
fn edit_crate_manifest(content: &str, identity: &ProjectIdentity) -> Option<String> {
    let mut doc: DocumentMut = content.parse().ok()?;
    doc["package"]["name"] = value(identity.name.as_str());
    doc["package"]["description"] = value(identity.description.as_str());
    let mut out = doc.to_string();
    let snake = identity.name_snake();
    out = out.replace("example-crate", &identity.name);
    out = out.replace("example_crate", &snake);
    Some(out)
}

/// Writes the profile's default CI tier into `config/xtask.json` content.
fn edit_ci_config(content: &str, tier: &str) -> Result<String, XtaskError> {
    let mut parsed: serde_json::Value =
        serde_json::from_str(content).map_err(|e| XtaskError::InvalidConfig {
            message: format!("config/xtask.json is not valid JSON: {e}"),
        })?;
    parsed["default_tier"] = serde_json::Value::String(tier.to_string());
    serde_json::to_string_pretty(&parsed).map_err(|e| XtaskError::InvalidConfig {
        message: format!("failed to serialize config/xtask.json: {e}"),
    })
}

/// Removes the lockfile-ignore block when the policy commits `Cargo.lock`.
fn edit_gitignore(content: &str) -> Option<String> {
    if !content.lines().any(|line| line == "Cargo.lock") {
        return None;
    }
    let filtered: Vec<&str> = content
        .lines()
        .filter(|line| !LOCKFILE_IGNORE_LINES.contains(&line.trim_end()))
        .collect();
    let mut out = filtered.join("\n");
    if content.ends_with('\n') && !out.is_empty() {
        out.push('\n');
    }
    Some(out)
}

/// Fixed-allowlist prose rewrites (doc files, examples, benchmarks).
/// Targets are code constants — never profile- or caller-controlled.
fn prose_rewrites(
    root: &Path,
    rename: Option<&CrateRename>,
    identity: &ProjectIdentity,
) -> Vec<FileRewrite> {
    let name = identity.name.as_str();
    let snake = identity.name_snake();
    let repo_url = format!("https://github.com/{}", identity.repo);
    let template_url = "https://github.com/d-oit/rust-2026-template";

    let mut rewrites = Vec::new();
    let mut push = |rel: &str, pairs: Vec<(&str, String)>| {
        let path = root.join(rel);
        if let Ok(content) = std::fs::read_to_string(&path) {
            let mut updated = content.clone();
            for (from, to) in pairs {
                updated = updated.replace(from, &to);
            }
            if updated != content {
                rewrites.push(FileRewrite {
                    path,
                    content: updated,
                });
            }
        }
    };

    push("AGENTS.md", vec![("rust-2026-template", name.to_string())]);
    push("README.md", vec![(template_url, repo_url)]);
    for doc_file in [
        "CLAUDE.md",
        "GEMINI.md",
        "QWEN.md",
        "CONTRIBUTING.md",
        "SECURITY.md",
        "QUICKSTART.md",
    ] {
        push(doc_file, vec![("rust-2026-template", name.to_string())]);
    }
    push(
        "examples/hello_world/Cargo.toml",
        vec![("example-crate", name.to_string())],
    );
    push(
        "examples/hello_world/src/main.rs",
        vec![
            ("example-crate", name.to_string()),
            ("example_crate", snake.clone()),
        ],
    );
    push(
        "benchmarks/Cargo.toml",
        vec![("example-crate", name.to_string())],
    );
    if let Some(r) = rename {
        for rel in ["src/lib.rs", "README.md"] {
            let from = r.from.join(rel);
            let to = r.to.join(rel);
            if let Ok(content) = std::fs::read_to_string(&from) {
                let updated = content
                    .replace("example-crate", name)
                    .replace("example_crate", &snake);
                if updated != content {
                    rewrites.push(FileRewrite {
                        path: to,
                        content: updated,
                    });
                }
            }
        }
    }
    rewrites
}
