//! Shared path/identifier validation rules for profiles and initialization.
//!
//! These rules guard every filesystem path that template tooling derives from
//! profile fields or caller input: normalized-relative containment, crate
//! directory names, profile ids, and package names.

/// Maximum byte length accepted for any profile-relative path entry.
const MAX_PATH_ENTRY_LEN: usize = 512;

/// Validates a profile id as a safe identifier (`^[a-z][a-z0-9-]{0,63}$`).
///
/// This runs *before* any filesystem path is derived from the id.
///
/// # Errors
/// Returns a human-readable reason when the id is not a safe identifier.
pub(crate) fn validate_profile_id_str(id: &str) -> Result<(), String> {
    let valid = !id.is_empty()
        && id.len() <= 64
        && id.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !valid {
        return Err(format!("'{id}' must match `^[a-z][a-z0-9-]{{0,63}}$`"));
    }
    Ok(())
}

/// True when `name` is a valid crate directory name under `crates/`.
pub(crate) fn is_crate_dir_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.starts_with(|c: char| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// True when `p` is a normalized relative path: no `..`, `.`, absolute root,
/// platform prefix, backslash, control characters, or excessive length.
pub(crate) fn is_safe_relative(p: &str) -> bool {
    if p.is_empty()
        || p.len() > MAX_PATH_ENTRY_LEN
        || p.contains('\\')
        || p.chars().any(char::is_control)
    {
        return false;
    }
    let path = std::path::Path::new(p);
    !path.is_absolute()
        && path
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
}

/// Validates a `workspace.include_crates` entry as `crates/<crate-name>`.
pub(crate) fn validate_include_crate(entry: &str) -> Result<(), String> {
    let rest = entry
        .strip_prefix("crates/")
        .ok_or_else(|| format!("'{entry}' must start with 'crates/'"))?;
    let mut components = std::path::Path::new(rest).components();
    match (components.next(), components.next()) {
        (Some(std::path::Component::Normal(name)), None) => {
            let name = name.to_str().unwrap_or("");
            if is_crate_dir_name(name) {
                Ok(())
            } else {
                Err(format!(
                    "'{entry}' must be `crates/<name>` where `<name>` matches `[a-z][a-z0-9-]*`"
                ))
            }
        }
        _ => Err(format!(
            "'{entry}' must be `crates/<name>` with exactly one path component"
        )),
    }
}

/// Validates a `workspace.exclude_paths` entry as a contained relative path.
pub(crate) fn validate_exclude_path(entry: &str) -> Result<(), String> {
    if is_safe_relative(entry) {
        Ok(())
    } else {
        Err(format!(
            "'{entry}' must be a relative path without '..', absolute components, backslashes, or control characters"
        ))
    }
}

/// Validates a `workspace.exclude_workflows` entry as a single `.yml` file name.
pub(crate) fn validate_exclude_workflow(entry: &str) -> Result<(), String> {
    if !is_safe_relative(entry) {
        return Err(format!(
            "'{entry}' must be a relative file name without '..'"
        ));
    }
    if std::path::Path::new(entry).components().count() != 1
        || !std::path::Path::new(entry)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("yml"))
    {
        return Err(format!(
            "'{entry}' must be a single workflow file name ending in '.yml'"
        ));
    }
    Ok(())
}

/// Validates a `policy.publish_packages` entry as a crate/package name.
pub(crate) fn validate_package_name(entry: &str) -> Result<(), String> {
    if entry.is_empty()
        || entry.len() > 64
        || !entry
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic())
        || !entry
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!(
            "'{entry}' must be a valid package name ([A-Za-z][A-Za-z0-9_-]{{0,63}})"
        ));
    }
    Ok(())
}
