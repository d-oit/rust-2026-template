//! Template initialization logic.
#![allow(clippy::unwrap_used)]

use crate::config::XtaskError;
use std::fs::{read_to_string, remove_dir_all, remove_file, write};
use std::path::Path;

/// Run the template initialization.
///
/// # Errors
/// Returns `XtaskError` if reading/writing/deleting files or folders fails.
pub fn run_init(
    profile: &str,
    name: Option<&str>,
    description: Option<&str>,
    author: Option<&str>,
    repo: Option<&str>,
    dry_run: bool,
) -> Result<(), XtaskError> {
    println!("==> Initializing template with profile: {profile}");

    let proj_name = name.unwrap_or("my-app");
    let proj_desc = description.unwrap_or("A production-ready Rust workspace");
    let proj_author = author.unwrap_or("Author");
    let proj_repo = repo.unwrap_or("myorg/my-app");

    if dry_run {
        println!("  [DRY RUN] Would initialize project '{proj_name}' ({proj_desc})");
        return Ok(());
    }

    // 1. If minimal profile, remove optional crates and workflows
    if profile == "minimal" {
        println!("  -> Applying minimal profile...");
        let optional_crates = &[
            "actor-runtime-template",
            "checkpoint-template",
            "hybrid-storage-template",
            "mcp-server-template",
            "example-registry-pattern",
            "example-storage-pattern",
        ];
        for crate_name in optional_crates {
            let path = format!("crates/{crate_name}");
            let p = Path::new(&path);
            if p.exists() {
                remove_dir_all(p).map_err(|e| XtaskError::CacheIssue { message: e.to_string() })?;
                println!("     Removed {path}");
            }
        }

        let optional_workflows = &[
            "dora-fdrt.yml",
            "dora-report.yml",
            "eval.yml",
            "mutants.yml",
            "skills-evaluation.yml",
            "update-architecture-diagram.yml",
            "cleanup-ci-status.yml",
            "sync-labels.yml",
            "labeler.yml",
            "patch-release-on-label.yml",
            "deploy-docs.yml",
            "fuzz.yml",
        ];
        for wf in optional_workflows {
            let path = format!(".github/workflows/{wf}");
            let p = Path::new(&path);
            if p.exists() {
                remove_file(p).map_err(|e| XtaskError::CacheIssue { message: e.to_string() })?;
                println!("     Removed {path}");
            }
        }

        let optional_dirs = &["fuzz", "benchmarks", "docs/patterns", ".template"];
        for dir in optional_dirs {
            let p = Path::new(dir);
            if p.exists() {
                remove_dir_all(p).map_err(|e| XtaskError::CacheIssue { message: e.to_string() })?;
                println!("     Removed {dir}");
            }
        }

        // Adjust Cargo.toml workspace members
        let cargo_toml_path = Path::new("Cargo.toml");
        if cargo_toml_path.exists() {
            let mut content = read_to_string(cargo_toml_path).map_err(|e| XtaskError::CacheIssue { message: e.to_string() })?;
            content = content.replace(", \"benchmarks\"", "");
            content = content.replace("\"benchmarks\", ", "");
            write(cargo_toml_path, content).map_err(|e| XtaskError::CacheIssue { message: e.to_string() })?;
            println!("     Adjusted Cargo.toml workspace members");
        }
    }

    // 2. Rename example-crate to proj_name
    let example_crate_path = Path::new("crates/example-crate");
    let new_crate_path = format!("crates/{proj_name}");
    let new_crate_p = Path::new(&new_crate_path);
    if example_crate_path.exists() && !new_crate_p.exists() {
        std::fs::rename(example_crate_path, new_crate_p).map_err(|e| XtaskError::CacheIssue { message: e.to_string() })?;
        println!("  -> Renamed crates/example-crate to {new_crate_path}");
    }

    // 3. String replacements in files
    println!("  -> Performing string replacements...");
    replace_placeholder("Cargo.toml", "Your Name", proj_author)?;
    replace_placeholder("Cargo.toml", "your-org/your-repo", proj_repo)?;
    replace_placeholder("Cargo.toml", "https://github.com/your-org/your-repo", &format!("https://github.com/{proj_repo}"))?;

    let crate_cargo = format!("crates/{proj_name}/Cargo.toml");
    replace_placeholder(&crate_cargo, "example-crate", proj_name)?;

    let crate_lib = format!("crates/{proj_name}/src/lib.rs");
    replace_placeholder(&crate_lib, "example-crate", proj_name)?;
    replace_placeholder(&crate_lib, "example_crate", &proj_name.replace('-', "_"))?;

    let example_readme = format!("crates/{proj_name}/README.md");
    replace_placeholder(&example_readme, "example-crate", proj_name)?;

    replace_placeholder("AGENTS.md", "rust-2026-template", proj_name)?;
    replace_placeholder("README.md", "rust-2026-template", proj_name)?;
    replace_placeholder("README.md", "https://github.com/d-oit/rust-2026-template", &format!("https://github.com/{proj_repo}"))?;

    println!("  ✓ Template initialized successfully!");
    Ok(())
}

fn replace_placeholder(file_path: &str, from: &str, to: &str) -> Result<(), XtaskError> {
    let p = Path::new(file_path);
    if p.exists() {
        let content = read_to_string(p).map_err(|e| XtaskError::CacheIssue { message: e.to_string() })?;
        let updated = content.replace(from, to);
        write(p, updated).map_err(|e| XtaskError::CacheIssue { message: e.to_string() })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_placeholder() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_replace.txt");
        write(&path, "Hello placeholder!").unwrap();
        replace_placeholder(path.to_str().unwrap(), "placeholder", "world").unwrap();
        let content = read_to_string(&path).unwrap();
        assert_eq!(content, "Hello world!");
        let _ = std::fs::remove_file(path);
    }
}
