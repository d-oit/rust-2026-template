//! Input validation for template initialization.
//!
//! Every value that reaches the filesystem or a generated manifest is validated
//! here *before* any mutation happens. Rejection is total: a failed validation
//! aborts initialization with no partial state.

use crate::config::XtaskError;
use std::path::{Path, PathBuf};

/// Maximum byte length accepted for a project name.
const MAX_NAME_LEN: usize = 64;
/// Maximum byte length accepted for an author field.
const MAX_AUTHOR_LEN: usize = 128;
/// Maximum byte length accepted for a description field.
const MAX_DESCRIPTION_LEN: usize = 256;

/// Windows reserved device names (case-insensitive, extension-less).
const WINDOWS_RESERVED: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Validated caller-supplied identity for the generated project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectIdentity {
    /// New name of the renamed example crate (and project display name).
    pub name: String,
    /// Project description written into the renamed crate manifest.
    pub description: String,
    /// Author written into `[workspace.package].authors`.
    pub author: String,
    /// `owner/repo` slug written into repository metadata and prose.
    pub repo: String,
}

impl ProjectIdentity {
    /// Validates (and defaults) the caller-supplied identity fields.
    ///
    /// # Errors
    /// Returns `XtaskError::InvalidConfig` when any supplied value is unsafe
    /// (path separators, traversal, control characters, reserved names, or
    /// excessive length).
    pub(crate) fn new(
        name: Option<&str>,
        description: Option<&str>,
        author: Option<&str>,
        repo: Option<&str>,
    ) -> Result<Self, XtaskError> {
        let name = name.unwrap_or("my-app");
        let description = description.unwrap_or("A production-ready Rust workspace");
        let author = author.unwrap_or("Author");
        let repo = repo.unwrap_or("myorg/my-app");

        validate_project_name(name).map_err(|reason| XtaskError::InvalidConfig {
            message: format!("--name {reason}"),
        })?;
        validate_single_line(description, MAX_DESCRIPTION_LEN).map_err(|reason| {
            XtaskError::InvalidConfig {
                message: format!("--description {reason}"),
            }
        })?;
        validate_single_line(author, MAX_AUTHOR_LEN).map_err(|reason| {
            XtaskError::InvalidConfig {
                message: format!("--author {reason}"),
            }
        })?;
        validate_repo_slug(repo).map_err(|reason| XtaskError::InvalidConfig {
            message: format!("--repo {reason}"),
        })?;

        Ok(Self {
            name: name.to_string(),
            description: description.trim().to_string(),
            author: author.trim().to_string(),
            repo: repo.trim().to_string(),
        })
    }

    /// Snake-cased crate identifier derived from the project name.
    pub(crate) fn name_snake(&self) -> String {
        self.name.replace('-', "_")
    }
}

/// Validates a project/crate name: ASCII, starts with a letter, contains only
/// `[A-Za-z0-9_-]`, no Windows reserved names, length 1..=64.
///
/// # Errors
/// Returns a human-readable reason when the name is unsafe.
pub fn validate_project_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > MAX_NAME_LEN {
        return Err(format!("'{name}' must be 1..={MAX_NAME_LEN} bytes"));
    }
    let first_ok = name.chars().next().is_some_and(|c| c.is_ascii_alphabetic());
    if !first_ok
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!(
            "'{name}' must start with a letter and contain only [A-Za-z0-9_-]"
        ));
    }
    if WINDOWS_RESERVED.contains(&name.to_ascii_lowercase().as_str()) {
        return Err(format!("'{name}' is a reserved Windows device name"));
    }
    Ok(())
}

/// Validates a `owner/repo` slug (GitHub-style, two non-empty segments).
///
/// # Errors
/// Returns a human-readable reason when the slug is malformed.
pub fn validate_repo_slug(repo: &str) -> Result<(), String> {
    let segments: Vec<&str> = repo.split('/').collect();
    if segments.len() != 2 {
        return Err(format!("'{repo}' must be exactly 'owner/repo'"));
    }
    for (label, segment) in [("owner", segments[0]), ("repo", segments[1])] {
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || segment.contains('\\')
            || segment.chars().any(char::is_control)
            || segment.len() > 100
            || !segment
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        {
            return Err(format!("'{repo}' has an invalid {label} segment"));
        }
    }
    Ok(())
}

/// Validates a single-line text field: trimmed non-empty, bounded, no control
/// characters (which could corrupt generated manifests).
///
/// # Errors
/// Returns a human-readable reason when the value is unsafe.
fn validate_single_line(value: &str, max_len: usize) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("'{value}' must not be empty"));
    }
    if trimmed.len() > max_len {
        return Err(format!("'{value}' must be at most {max_len} bytes"));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(format!("'{value}' must not contain control characters"));
    }
    Ok(())
}

/// Canonicalizes `path` and proves it resolves inside `root`.
///
/// Symlinked targets (or ancestors) that escape `root` are rejected. The path
/// must exist.
///
/// # Errors
/// Returns `XtaskError::InvalidConfig` when the path does not exist or
/// resolves outside `root`.
pub fn contained_canonical(path: &Path, root: &Path) -> Result<PathBuf, XtaskError> {
    let canonical_root = root.canonicalize().map_err(|e| XtaskError::InvalidConfig {
        message: format!("repository root {} is not accessible: {e}", root.display()),
    })?;
    let canonical = path.canonicalize().map_err(|e| XtaskError::InvalidConfig {
        message: format!(
            "path {} does not exist or is not accessible: {e}",
            path.display()
        ),
    })?;
    if !canonical.starts_with(&canonical_root) {
        return Err(XtaskError::InvalidConfig {
            message: format!(
                "path {} resolves outside the repository root (symlink escape?)",
                path.display()
            ),
        });
    }
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]
    use super::*;

    #[test]
    fn identity_defaults_are_valid() {
        let id = ProjectIdentity::new(None, None, None, None).unwrap();
        assert_eq!(id.name, "my-app");
        assert_eq!(id.name_snake(), "my_app");
        assert_eq!(id.repo, "myorg/my-app");
    }

    #[test]
    fn identity_rejects_traversal_and_separators() {
        for bad in [
            "../outside",
            "a/b",
            "a\\b",
            "/abs",
            ".",
            "..",
            "",
            "1app",
            "my app",
            "CON",
            "nul",
        ] {
            let err = ProjectIdentity::new(Some(bad), None, None, None).unwrap_err();
            assert!(
                err.to_string().contains("--name"),
                "'{bad}' rejected: {err}"
            );
        }
    }

    #[test]
    fn identity_accepts_valid_names() {
        for ok in ["my-app", "my_app", "App2", "web-server-2026"] {
            assert!(
                ProjectIdentity::new(Some(ok), None, None, None).is_ok(),
                "'{ok}' must be accepted"
            );
        }
    }

    #[test]
    fn identity_rejects_bad_repo_slugs() {
        for bad in [
            "",
            "onlyone",
            "a/b/c",
            "../etc",
            "org/../x",
            "a b/c",
            "org/repo\n",
            "/abs/rel",
            "org//repo2",
        ] {
            let err = ProjectIdentity::new(None, None, None, Some(bad)).unwrap_err();
            assert!(
                err.to_string().contains("--repo"),
                "'{bad}' rejected: {err}"
            );
        }
    }

    #[test]
    fn identity_rejects_controls_and_oversize_text() {
        assert!(ProjectIdentity::new(None, Some("bad\u{7}desc"), None, None).is_err());
        assert!(ProjectIdentity::new(None, None, Some("  "), None).is_err());
        let long = "x".repeat(300);
        assert!(ProjectIdentity::new(None, Some(&long), None, None).is_err());
    }

    #[test]
    fn contained_canonical_rejects_escape() {
        let root = tempfile::tempdir().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let inside = root.path().join("dir");
        std::fs::create_dir_all(&inside).unwrap();
        let canonical = contained_canonical(&inside, root.path()).unwrap();
        assert!(canonical.starts_with(root.path().canonicalize().unwrap()));

        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        let err = contained_canonical(&outside, root.path()).unwrap_err();
        assert!(err.to_string().contains("outside the repository root"));
    }

    #[cfg(unix)]
    #[test]
    fn contained_canonical_rejects_symlink_escape() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("real")).unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("link")).unwrap();
        let err = contained_canonical(&root.path().join("link"), root.path()).unwrap_err();
        assert!(err.to_string().contains("outside the repository root"));
    }
}
