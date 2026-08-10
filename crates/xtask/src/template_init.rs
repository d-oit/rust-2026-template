//! Template initialization logic (profile-driven, issue #286).
//!
//! A `cargo xtask template init --profile <id>` loads the matching blueprint from
//! `config/template-profiles/`, validates it, shapes the workspace (remove unselected
//! crates, paths, workflows), renames the example crate, applies placeholder
//! replacements, writes the profile's CI-tier default, and prints the post-init checklist.

use crate::config::XtaskError;
use crate::template_profile::TemplateProfile;
use std::fs::{read_dir, read_to_string, remove_dir_all, remove_file, write};
use std::path::Path;

/// Run the template initialization using a validated profile blueprint.
///
/// # Errors
/// Returns `XtaskError` if the profile is unknown/invalid or any file operation fails.
pub fn run_init(
    profile: &str,
    name: Option<&str>,
    description: Option<&str>,
    author: Option<&str>,
    repo: Option<&str>,
    dry_run: bool,
) -> Result<(), XtaskError> {
    let blueprint = TemplateProfile::load(profile)?;
    println!(
        "==> Initializing template with profile: {}",
        blueprint.metadata.id
    );

    let proj_name = name.unwrap_or("my-app");
    let proj_desc = description.unwrap_or("A production-ready Rust workspace");
    let proj_author = author.unwrap_or("Author");
    let proj_repo = repo.unwrap_or("myorg/my-app");

    if dry_run {
        println!("  [DRY RUN] Would initialize project '{proj_name}' ({proj_desc})");
        let existing = existing_crates()?;
        let removed = blueprint.removed_crates(&existing);
        println!(
            "  [DRY RUN] Would remove {} crate(s): {removed:?}",
            removed.len()
        );
        for p in &blueprint.workspace.exclude_paths {
            println!("  [DRY RUN] Would remove path: {p}");
        }
        for wf in &blueprint.workspace.exclude_workflows {
            println!("  [DRY RUN] Would remove workflow: .github/workflows/{wf}");
        }
        println!(
            "  [DRY RUN] Would set default CI tier: {}",
            blueprint.ci.default_tier
        );
        return Ok(());
    }

    apply_profile(&blueprint)?;
    rename_example_crate(proj_name)?;
    perform_replacements(proj_name, proj_author, proj_repo)?;
    apply_ci_tier(&blueprint)?;
    print_post_init(&blueprint);

    println!(
        "  ✓ Template initialized successfully with profile '{}'!",
        blueprint.metadata.id
    );
    Ok(())
}

/// Applies the profile's workspace-shaping decisions (removals only; no generation).
fn apply_profile(blueprint: &TemplateProfile) -> Result<(), XtaskError> {
    println!("  -> Applying '{}' profile...", blueprint.metadata.id);

    let existing = existing_crates()?;
    for removed in blueprint.removed_crates(&existing) {
        let path = format!("crates/{removed}");
        if Path::new(&path).exists() {
            remove_dir_all(&path).map_err(|e| XtaskError::CacheIssue {
                message: e.to_string(),
            })?;
            println!("     Removed {path}");
        }
    }

    for excluded in &blueprint.workspace.exclude_paths {
        remove_path(excluded)?;
    }

    for wf in &blueprint.workspace.exclude_workflows {
        let path = format!(".github/workflows/{wf}");
        remove_path(&path)?;
    }

    // Drop `benchmarks` from workspace members when the profile excludes it.
    if blueprint
        .workspace
        .exclude_paths
        .iter()
        .any(|p| p == "benchmarks")
    {
        let cargo_toml_path = Path::new("Cargo.toml");
        if cargo_toml_path.exists() {
            let mut content =
                read_to_string(cargo_toml_path).map_err(|e| XtaskError::CacheIssue {
                    message: e.to_string(),
                })?;
            content = content.replace(", \"benchmarks\"", "");
            content = content.replace("\"benchmarks\", ", "");
            write(cargo_toml_path, content).map_err(|e| XtaskError::CacheIssue {
                message: e.to_string(),
            })?;
            println!("     Adjusted Cargo.toml workspace members");
        }
    }

    Ok(())
}

/// Writes the profile's `ci.default_tier` into `config/xtask.json`.
fn apply_ci_tier(blueprint: &TemplateProfile) -> Result<(), XtaskError> {
    let path = Path::new("config/xtask.json");
    if !path.exists() {
        println!(
            "     ! config/xtask.json not found; skipping CI tier default (profile: {})",
            blueprint.ci.default_tier
        );
        return Ok(());
    }
    let content = read_to_string(path).map_err(|e| XtaskError::CacheIssue {
        message: e.to_string(),
    })?;
    let mut value: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| XtaskError::InvalidConfig {
            message: e.to_string(),
        })?;
    value["default_tier"] = serde_json::Value::String(blueprint.ci.default_tier.clone());
    let updated = serde_json::to_string_pretty(&value).map_err(|e| XtaskError::InvalidConfig {
        message: e.to_string(),
    })?;
    write(path, updated).map_err(|e| XtaskError::CacheIssue {
        message: e.to_string(),
    })?;
    println!(
        "     Set config/xtask.json default tier: {}",
        blueprint.ci.default_tier
    );
    Ok(())
}

/// Prints the profile's post-init checklist (items that cannot travel through a GitHub template).
fn print_post_init(blueprint: &TemplateProfile) {
    if blueprint.post_init.checklist.is_empty() {
        return;
    }
    println!();
    println!("  ── Post-init checklist ──");
    for item in &blueprint.post_init.checklist {
        println!("     - [ ] {item}");
    }
}

/// Lists top-level crate directory names under `crates/`.
fn existing_crates() -> Result<Vec<String>, XtaskError> {
    let mut names = Vec::new();
    let entries = read_dir("crates").map_err(|e| XtaskError::CacheIssue {
        message: e.to_string(),
    })?;
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

/// Removes a path whether it is a file, directory, or already absent (idempotent).
fn remove_path(p: &str) -> Result<(), XtaskError> {
    let path = Path::new(p);
    if path.is_dir() {
        remove_dir_all(path).map_err(|e| XtaskError::CacheIssue {
            message: e.to_string(),
        })?;
        println!("     Removed {p}");
    } else if path.is_file() {
        remove_file(path).map_err(|e| XtaskError::CacheIssue {
            message: e.to_string(),
        })?;
        println!("     Removed {p}");
    }
    Ok(())
}

fn rename_example_crate(proj_name: &str) -> Result<(), XtaskError> {
    let example_crate_path = Path::new("crates/example-crate");
    let new_crate_path = format!("crates/{proj_name}");
    let new_crate_p = Path::new(&new_crate_path);
    if example_crate_path.exists() && !new_crate_p.exists() {
        std::fs::rename(example_crate_path, new_crate_p).map_err(|e| XtaskError::CacheIssue {
            message: e.to_string(),
        })?;
        println!("  -> Renamed crates/example-crate to {new_crate_path}");
    }
    Ok(())
}

fn perform_replacements(
    proj_name: &str,
    proj_author: &str,
    proj_repo: &str,
) -> Result<(), XtaskError> {
    println!("  -> Performing string replacements...");
    let proj_snake = proj_name.replace('-', "_");
    replace_placeholder("Cargo.toml", "Your Name", proj_author)?;
    replace_placeholder("Cargo.toml", "your-org/your-repo", proj_repo)?;
    replace_placeholder(
        "Cargo.toml",
        "https://github.com/your-org/your-repo",
        &format!("https://github.com/{proj_repo}"),
    )?;

    let crate_cargo = format!("crates/{proj_name}/Cargo.toml");
    replace_placeholder(&crate_cargo, "example-crate", proj_name)?;

    let crate_lib = format!("crates/{proj_name}/src/lib.rs");
    replace_placeholder(&crate_lib, "example-crate", proj_name)?;
    replace_placeholder(&crate_lib, "example_crate", &proj_name.replace('-', "_"))?;

    let example_readme = format!("crates/{proj_name}/README.md");
    replace_placeholder(&example_readme, "example-crate", proj_name)?;
    replace_placeholder(&example_readme, "example_crate", &proj_snake)?;

    replace_placeholder("AGENTS.md", "rust-2026-template", proj_name)?;
    replace_placeholder(
        "README.md",
        "https://github.com/d-oit/rust-2026-template",
        &format!("https://github.com/{proj_repo}"),
    )?;

    // Tool-adapter and contribution docs carry the template name — rename them too so a
    // generated project has no stale references.
    for doc in [
        "CLAUDE.md",
        "GEMINI.md",
        "QWEN.md",
        "CONTRIBUTING.md",
        "SECURITY.md",
        "QUICKSTART.md",
    ] {
        replace_placeholder(doc, "rust-2026-template", proj_name)?;
    }

    // Examples/benchmarks that reference the renamed example crate keep the workspace buildable.
    replace_placeholder(
        "examples/hello_world/Cargo.toml",
        "example-crate",
        proj_name,
    )?;
    replace_placeholder(
        "examples/hello_world/src/main.rs",
        "example-crate",
        proj_name,
    )?;
    replace_placeholder(
        "examples/hello_world/src/main.rs",
        "example_crate",
        &proj_snake,
    )?;
    replace_placeholder("benchmarks/Cargo.toml", "example-crate", proj_name)?;

    Ok(())
}

fn replace_placeholder(file_path: &str, from: &str, to: &str) -> Result<(), XtaskError> {
    let p = Path::new(file_path);
    if p.exists() {
        let content = read_to_string(p).map_err(|e| XtaskError::CacheIssue {
            message: e.to_string(),
        })?;
        let updated = content.replace(from, to);
        write(p, updated).map_err(|e| XtaskError::CacheIssue {
            message: e.to_string(),
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn test_replace_placeholder() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("test_replace.txt");
        write(&path, "Hello placeholder!").unwrap();
        replace_placeholder(path.to_str().unwrap(), "placeholder", "world").unwrap();
        let content = read_to_string(&path).unwrap();
        assert_eq!(content, "Hello world!");
    }
}
