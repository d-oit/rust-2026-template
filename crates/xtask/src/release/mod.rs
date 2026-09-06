//! Release automation: workspace patch-version bumping.
//!
//! [`bump_patch`] is the single mutation primitive consumed by the
//! `patch-release-on-label.yml` workflow. It validates the workspace manifest
//! and the optional plain-text `VERSION` sentinel file completely, then bumps
//! the patch segment in place, preserving all other manifest formatting. Every
//! check runs before the first write, so a rejected bump mutates nothing.

use crate::config::XtaskError;
use std::path::Path;

/// Default workspace manifest path, relative to the repository root.
pub const DEFAULT_MANIFEST: &str = "Cargo.toml";

/// Default plain-text version file path, relative to the repository root.
pub const DEFAULT_VERSION_FILE: &str = "VERSION";

/// Bump the patch segment of the workspace version in `root`.
///
/// Reads `root/Cargo.toml`, parses it with `toml_edit`, requires
/// `[workspace.package].version` to be exactly `X.Y.Z` (three numeric
/// segments), and produces `X.Y.(Z+1)` with all other formatting preserved.
/// When `root/VERSION` exists it must record the pre-bump version; a stale or
/// malformed sentinel rejects the whole operation with zero mutation. On
/// success the sentinel is rewritten to the new version plus a trailing
/// newline; when it is absent only the manifest is touched.
///
/// With `dry_run` the function validates and reports the computed bump but
/// writes nothing.
///
/// # Errors
/// Returns `XtaskError::InvalidConfig` when the manifest is missing,
/// unparsable, has no string `[workspace.package].version`, the version is
/// not `X.Y.Z`, the patch segment overflows, or the `VERSION` file exists but
/// disagrees with the pre-bump manifest version.
pub fn bump_patch(root: &Path, dry_run: bool) -> Result<String, XtaskError> {
    bump_patch_in(root, DEFAULT_MANIFEST, DEFAULT_VERSION_FILE, dry_run)
}

/// Like [`bump_patch`] but with explicit manifest/version-file paths.
///
/// `manifest` and `version_file` are resolved relative to `root`. The
/// command-line layer uses this to honour `--file` / `--version-file`
/// overrides; the defaults are [`DEFAULT_MANIFEST`] and
/// [`DEFAULT_VERSION_FILE`].
///
/// # Errors
/// Same contract as [`bump_patch`].
pub fn bump_patch_in(
    root: &Path,
    manifest: &str,
    version_file: &str,
    dry_run: bool,
) -> Result<String, XtaskError> {
    let manifest_path = root.join(manifest);
    let version_path = root.join(version_file);

    // Validation phase: everything is checked before the first write.
    let manifest_bytes = std::fs::read(&manifest_path).map_err(|e| XtaskError::InvalidConfig {
        message: format!("cannot read manifest '{}': {e}", manifest_path.display()),
    })?;
    let manifest_text =
        String::from_utf8(manifest_bytes).map_err(|e| XtaskError::InvalidConfig {
            message: format!(
                "manifest '{}' is not valid UTF-8: {e}",
                manifest_path.display()
            ),
        })?;
    let mut doc = manifest_text
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| XtaskError::InvalidConfig {
            message: format!(
                "manifest '{}' is not valid TOML: {e}",
                manifest_path.display()
            ),
        })?;

    let (current, next) = validated_version_pair(&doc, manifest)?;
    let version_file_present = version_path.exists();
    let version_content = if version_file_present {
        Some(
            std::fs::read_to_string(&version_path).map_err(|e| XtaskError::InvalidConfig {
                message: format!("cannot read version file '{}': {e}", version_path.display()),
            })?,
        )
    } else {
        None
    };
    if let Some(content) = &version_content {
        let recorded = content.trim();
        if recorded != current {
            return Err(XtaskError::InvalidConfig {
                message: format!(
                    "version file '{}' is stale: it records '{recorded}' but the manifest \
                     version is '{current}'; refusing to mutate anything",
                    version_path.display()
                ),
            });
        }
    }

    let package = doc
        .as_table_mut()
        .get_mut("workspace")
        .and_then(toml_edit::Item::as_table_mut)
        .and_then(|ws| ws.get_mut("package"))
        .and_then(toml_edit::Item::as_table_mut)
        .ok_or_else(|| XtaskError::InvalidConfig {
            message: format!("manifest '{manifest}' has no [workspace.package] table"),
        })?;
    let version_item = package
        .get_mut("version")
        .ok_or_else(|| XtaskError::InvalidConfig {
            message: format!("[workspace.package] in '{manifest}' has no 'version' key"),
        })?;
    *version_item = toml_edit::value(next.as_str());

    if dry_run {
        if version_file_present {
            println!(
                "DRY-RUN: would bump '{manifest}' version {current} -> {next} and update '{version_file}'"
            );
        } else {
            println!(
                "DRY-RUN: would bump '{manifest}' version {current} -> {next} ('{version_file}' absent, manifest only)"
            );
        }
        return Ok(next);
    }

    std::fs::write(&manifest_path, doc.to_string()).map_err(|e| XtaskError::InvalidConfig {
        message: format!("cannot write manifest '{}': {e}", manifest_path.display()),
    })?;
    println!("Bumped '{manifest}' version {current} -> {next}");
    if version_file_present {
        std::fs::write(&version_path, format!("{next}\n")).map_err(|e| {
            XtaskError::InvalidConfig {
                message: format!(
                    "cannot write version file '{}': {e}",
                    version_path.display()
                ),
            }
        })?;
        println!("Updated '{version_file}' ({current} -> {next})");
    }
    Ok(next)
}

/// Extracts `[workspace.package].version` and its patch-bumped successor.
///
/// The version must be exactly `X.Y.Z` — three dot-separated, all-digit
/// segments parseable as `u64`.
fn validated_version_pair(
    doc: &toml_edit::DocumentMut,
    manifest: &str,
) -> Result<(String, String), XtaskError> {
    let invalid = |message: String| XtaskError::InvalidConfig { message };
    let version = doc
        .as_table()
        .get("workspace")
        .and_then(toml_edit::Item::as_table)
        .and_then(|ws| ws.get("package"))
        .and_then(toml_edit::Item::as_table)
        .and_then(|pkg| pkg.get("version"))
        .and_then(toml_edit::Item::as_str)
        .ok_or_else(|| {
            invalid(format!(
                "manifest '{manifest}' must define a string '[workspace.package].version'"
            ))
        })?;
    let segments: Vec<&str> = version.split('.').collect();
    if segments.len() != 3 {
        return Err(invalid(format!(
            "manifest '{manifest}' version '{version}' must be X.Y.Z (exactly three segments)"
        )));
    }
    let mut numbers = [0_u64; 3];
    for (index, segment) in segments.iter().enumerate() {
        let is_numeric = !segment.is_empty() && segment.bytes().all(|b| b.is_ascii_digit());
        if !is_numeric {
            return Err(invalid(format!(
                "manifest '{manifest}' version '{version}' must be X.Y.Z; segment \
                 '{segment}' is not numeric"
            )));
        }
        numbers[index] = segment.parse::<u64>().map_err(|_| {
            invalid(format!(
                "manifest '{manifest}' version '{version}' has a segment exceeding u64"
            ))
        })?;
    }
    let patch = numbers[2]
        .checked_add(1)
        .ok_or_else(|| invalid("patch version segment overflows u64".to_string()))?;
    Ok((
        version.to_string(),
        format!("{}.{}.{patch}", numbers[0], numbers[1]),
    ))
}

/// Returns `true` when every changed path is explicitly allowlisted.
///
/// Used to gate CI pushes on the diff containing only release-relevant
/// files. An empty diff is allowlisted; paths must match an allow entry
/// exactly (no glob or prefix semantics).
#[must_use]
pub fn allowlisted(diff_paths: &[String], allow: &[&str]) -> bool {
    diff_paths.iter().all(|path| allow.contains(&path.as_str()))
}

/// `xtask release` subcommands (clap wiring lives here to keep `main.rs` lean).
#[derive(clap::Subcommand)]
pub enum ReleaseSub {
    /// Bump the workspace patch version (and the VERSION file when present).
    BumpPatch {
        /// Workspace manifest path, relative to the working directory.
        #[arg(long, default_value = "Cargo.toml")]
        file: String,
        /// Plain-text version file path, relative to the working directory.
        #[arg(long, default_value = "VERSION")]
        version_file: String,
        /// Compute and report the bump without writing anything.
        #[arg(long)]
        dry_run: bool,
    },
}

impl ReleaseSub {
    /// Dispatches the subcommand, resolving paths against the current directory.
    ///
    /// # Errors
    /// Returns `XtaskError` when the bump cannot be applied (see [`bump_patch_in`]).
    pub fn run(self) -> Result<(), XtaskError> {
        let Self::BumpPatch {
            file,
            version_file,
            dry_run,
        } = self;
        std::env::current_dir()
            .map_err(|e| XtaskError::InvalidConfig {
                message: format!("cannot determine working directory: {e}"),
            })
            .and_then(|dir| bump_patch_in(&dir, &file, &version_file, dry_run).map(drop))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]

    use super::*;

    const MANIFEST_V000: &str = "\
# workspace manifest
[workspace]
resolver = \"3\"

[workspace.package]
version = \"0.0.0\"
edition = \"2024\"
";

    fn write(root: &Path, rel: &str, content: &str) {
        std::fs::write(root.join(rel), content).unwrap();
    }

    #[test]
    fn bump_patch_bumps_manifest_and_syncs_version_file() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        write(root, "Cargo.toml", MANIFEST_V000);
        write(root, "VERSION", "0.0.0\n");

        let next = bump_patch(root, false).unwrap();

        assert_eq!(next, "0.0.1");
        let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
        assert_eq!(
            manifest,
            MANIFEST_V000.replace("version = \"0.0.0\"", "version = \"0.0.1\"")
        );
        assert_eq!(
            std::fs::read_to_string(root.join("VERSION")).unwrap(),
            "0.0.1\n"
        );
    }

    #[test]
    fn bump_patch_rejects_stale_version_file_without_mutation() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        write(root, "Cargo.toml", MANIFEST_V000);
        write(root, "VERSION", "9.9.9\n");
        let manifest_before = std::fs::read(root.join("Cargo.toml")).unwrap();
        let version_before = std::fs::read(root.join("VERSION")).unwrap();

        let err = bump_patch(root, false).unwrap_err();

        assert!(matches!(err, XtaskError::InvalidConfig { .. }));
        assert_eq!(
            std::fs::read(root.join("Cargo.toml")).unwrap(),
            manifest_before
        );
        assert_eq!(std::fs::read(root.join("VERSION")).unwrap(), version_before);
    }

    #[test]
    fn bump_patch_rejects_malformed_version_without_mutation() {
        for bad in [
            "0.0",
            "0.0.0.1",
            "0.0.beta",
            "one.two.three",
            "0.0.",
            "0.0.-1",
            "0.0.+5",
        ] {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path();
            write(root, "Cargo.toml", &MANIFEST_V000.replace("0.0.0", bad));
            let before = std::fs::read(root.join("Cargo.toml")).unwrap();

            let err = bump_patch(root, false).unwrap_err();

            assert!(
                matches!(err, XtaskError::InvalidConfig { .. }),
                "case {bad}"
            );
            assert_eq!(
                std::fs::read(root.join("Cargo.toml")).unwrap(),
                before,
                "case {bad}"
            );
        }
    }

    #[test]
    fn bump_patch_without_version_file_updates_manifest_only() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        write(root, "Cargo.toml", MANIFEST_V000);

        let next = bump_patch(root, false).unwrap();

        assert_eq!(next, "0.0.1");
        let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
        assert!(manifest.contains("version = \"0.0.1\""));
        assert!(!root.join("VERSION").exists());
    }

    #[test]
    fn bump_patch_dry_run_writes_nothing() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        write(root, "Cargo.toml", MANIFEST_V000);
        write(root, "VERSION", "0.0.0");
        let manifest_before = std::fs::read(root.join("Cargo.toml")).unwrap();
        let version_before = std::fs::read(root.join("VERSION")).unwrap();

        let next = bump_patch(root, true).unwrap();

        assert_eq!(next, "0.0.1");
        assert_eq!(
            std::fs::read(root.join("Cargo.toml")).unwrap(),
            manifest_before
        );
        assert_eq!(std::fs::read(root.join("VERSION")).unwrap(), version_before);
    }

    #[test]
    fn bump_patch_increments_patch_segment_of_nonzero_version() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        write(root, "Cargo.toml", &MANIFEST_V000.replace("0.0.0", "1.2.7"));

        let next = bump_patch(root, false).unwrap();

        assert_eq!(next, "1.2.8");
    }

    #[test]
    fn bump_patch_rejects_missing_workspace_package() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        write(root, "Cargo.toml", "[workspace]\nresolver = \"3\"\n");

        let err = bump_patch(root, false).unwrap_err();

        assert!(matches!(err, XtaskError::InvalidConfig { .. }));
    }

    #[test]
    fn allowlisted_gates_diff_paths() {
        let allow = ["Cargo.toml", "VERSION"];
        assert!(allowlisted(&[], &allow));
        assert!(allowlisted(
            &["Cargo.toml".to_string(), "VERSION".to_string()],
            &allow
        ));
        assert!(!allowlisted(
            &["Cargo.toml".to_string(), "src/lib.rs".to_string()],
            &allow
        ));
        assert!(!allowlisted(&["Cargo.toml".to_string()], &[]));
    }
}
