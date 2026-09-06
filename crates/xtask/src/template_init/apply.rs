//! Applies an `InitPlan`.
//!
//! Execution order: removals first (nothing else references those paths), then
//! the rename, then all prepared rewrites. Every rewrite goes through a
//! unique same-directory temporary file followed by a rename, so a crash
//! cannot leave a half-written file at the destination.

use super::plan::InitPlan;
use crate::config::XtaskError;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process;

/// Executes the plan. All validation happened at plan time.
///
/// # Errors
/// Returns `XtaskError::CacheIssue` when a filesystem operation fails.
pub fn execute(plan: &InitPlan) -> Result<(), XtaskError> {
    println!("  -> Applying '{}' profile...", plan.profile_id);

    for path in &plan.removals {
        remove_any(path)?;
    }

    if let Some(rename) = &plan.rename {
        fs::rename(&rename.from, &rename.to).map_err(|e| XtaskError::CacheIssue {
            message: format!(
                "failed to rename {} to {}: {e}",
                rename.from.display(),
                rename.to.display()
            ),
        })?;
        println!(
            "     Renamed crates/example-crate to crates/{}",
            plan.identity.name
        );
    }

    if let Some(rewrite) = &plan.workspace_manifest {
        write_atomic(&rewrite.path, &rewrite.content)?;
        println!("     Adjusted Cargo.toml workspace members and package identity");
    }
    if let Some(rewrite) = &plan.crate_manifest {
        write_atomic(&rewrite.path, &rewrite.content)?;
        println!(
            "     Set crate package name '{}' and description",
            plan.identity.name
        );
    }
    if let Some(rewrite) = &plan.gitignore {
        write_atomic(&rewrite.path, &rewrite.content)?;
        println!("     Cargo.lock is no longer ignored (policy: committed)");
    }
    for rewrite in &plan.text_replacements {
        write_atomic(&rewrite.path, &rewrite.content)?;
    }
    if !plan.text_replacements.is_empty() {
        println!(
            "     Rewrote {} doc/example file(s)",
            plan.text_replacements.len()
        );
    }
    if let Some(rewrite) = &plan.ci_config {
        write_atomic(&rewrite.path, &rewrite.content)?;
        println!(
            "     Set config/xtask.json default tier: {}",
            plan.default_tier
        );
    } else {
        println!(
            "     ! config/xtask.json not found; skipping CI tier default (profile: {})",
            plan.default_tier
        );
    }

    print_post_init(plan);
    Ok(())
}

/// Removes a path whether it is a file, directory, or already absent.
fn remove_any(path: &Path) -> Result<(), XtaskError> {
    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|e| XtaskError::CacheIssue {
            message: format!("failed to remove directory {}: {e}", path.display()),
        })?;
        println!("     Removed {}", path.display());
    } else if path.is_file() {
        fs::remove_file(path).map_err(|e| XtaskError::CacheIssue {
            message: format!("failed to remove file {}: {e}", path.display()),
        })?;
        println!("     Removed {}", path.display());
    }
    Ok(())
}

/// Writes `content` to `path` via a unique same-directory temp file + rename.
///
/// Rename-over-existing is used where the platform supports it; if rename
/// fails while the destination exists (Windows), the destination is removed
/// and the rename retried. Temp files use the `.xtask-tmp-<pid>` suffix and
/// are cleaned up on write failure.
fn write_atomic(path: &Path, content: &str) -> Result<(), XtaskError> {
    let file_name = path
        .file_name()
        .map_or_else(|| "file".to_string(), |n| n.to_string_lossy().into_owned());
    let tmp = path.with_file_name(format!(".{file_name}.xtask-tmp-{}", process::id()));

    let write_result = (|| -> std::io::Result<()> {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        drop(file);
        match fs::rename(&tmp, path) {
            Ok(()) => Ok(()),
            Err(_) if path.exists() => {
                fs::remove_file(path)?;
                fs::rename(&tmp, path)
            }
            Err(e) => Err(e),
        }
    })();

    if let Err(e) = write_result {
        let _ = fs::remove_file(&tmp);
        return Err(XtaskError::CacheIssue {
            message: format!("failed to atomically write {}: {e}", path.display()),
        });
    }
    Ok(())
}

/// Prints the profile's post-init checklist (items that cannot travel through
/// a GitHub template).
fn print_post_init(plan: &InitPlan) {
    if plan.checklist.is_empty() {
        return;
    }
    println!();
    println!("  ── Post-init checklist ──");
    for item in &plan.checklist {
        println!("     - [ ] {item}");
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]
    use super::*;

    #[test]
    fn write_atomic_creates_and_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.txt");

        write_atomic(&path, "first").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "first");

        write_atomic(&path, "second").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "second");

        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains("xtask-tmp"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "temp files must not leak: {leftovers:?}"
        );
    }

    #[test]
    fn write_atomic_failure_cleans_up() {
        // A directory at the temp-file location forces create() to fail.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("blocked.txt");
        let tmp = path.with_file_name(format!(".blocked.txt.xtask-tmp-{}", process::id()));
        fs::create_dir(&tmp).unwrap();

        assert!(write_atomic(&path, "x").is_err());
        // The pre-existing temp dir is not ours to remove; destination untouched.
        assert!(!path.exists());
    }

    #[test]
    fn remove_any_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("gone.txt");
        fs::write(&file, "x").unwrap();
        remove_any(&file).unwrap();
        assert!(!file.exists());
        remove_any(&file).unwrap();
    }
}
