//! Validation-focused tests for `template_profile` (kept out-of-line to respect the 500-LOC limit).
#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
use crate::path_rules::{is_crate_dir_name, is_safe_relative};

/// Builds a profile TOML with the given workspace section body.
fn profile_toml_with_workspace(workspace_body: &str) -> String {
    format!(
        r#"[metadata]
id = "t"
display_name = "T"
description = "d"
[workspace]
{workspace_body}
[ci]
default_tier = "pull-request"
[post_init]
checklist = ["x"]
"#
    )
}

#[test]
fn test_load_rejects_profile_id_with_traversal() {
    for bad in ["../evil", "a/b", "a\\b", "/abs", "..", ".", "con"] {
        let err = TemplateProfile::load(bad).unwrap_err();
        assert!(
            err.to_string().contains("invalid profile id"),
            "profile id '{bad}' must be rejected by id validation, got: {err}"
        );
    }
}

#[test]
fn test_validate_rejects_include_crates_traversal() {
    for bad in [
        "crates/../Cargo.toml",
        "crates/a/b",
        "crates/",
        "crates/../..",
        "/crates/x",
    ] {
        let toml = profile_toml_with_workspace(&format!("include_crates = [\"{bad}\"]"));
        let err = TemplateProfile::from_toml(&toml).unwrap_err();
        assert!(
            err.to_string().contains("include_crates"),
            "include entry '{bad}' must be rejected, got: {err}"
        );
    }
}

#[test]
fn test_validate_rejects_exclude_paths_traversal() {
    for bad in ["..", "/abs", "a/../b", "", "a\\b", ".", "sub/../.."] {
        let toml = profile_toml_with_workspace(&format!(
            "include_crates = [\"crates/xtask\"]\nexclude_paths = [\"{bad}\"]"
        ));
        let err = TemplateProfile::from_toml(&toml).unwrap_err();
        assert!(
            err.to_string().contains("exclude_paths"),
            "exclude path '{bad}' must be rejected, got: {err}"
        );
    }
}

#[test]
fn test_validate_rejects_exclude_workflows_bad() {
    for bad in ["../x.yml", "sub/x.yml", "x.sh", "..", ""] {
        let toml = profile_toml_with_workspace(&format!(
            "include_crates = [\"crates/xtask\"]\nexclude_workflows = [\"{bad}\"]"
        ));
        let err = TemplateProfile::from_toml(&toml).unwrap_err();
        assert!(
            err.to_string().contains("exclude_workflows"),
            "exclude workflow '{bad}' must be rejected, got: {err}"
        );
    }
}

#[test]
fn test_validate_rejects_metadata_id_leading_digit() {
    let toml = profile_toml_with_workspace("include_crates = [\"crates/xtask\"]")
        .replace("id = \"t\"", "id = \"1bad\"");
    let err = TemplateProfile::from_toml(&toml).unwrap_err();
    assert!(err.to_string().contains("metadata.id"));
}

#[test]
fn test_is_safe_relative_rejects_component_attacks() {
    assert!(is_safe_relative("benchmarks"));
    assert!(is_safe_relative("docs/patterns"));
    assert!(!is_safe_relative(""));
    assert!(!is_safe_relative(".."));
    assert!(!is_safe_relative("."));
    assert!(!is_safe_relative("/abs"));
    assert!(!is_safe_relative("a/../b"));
    assert!(!is_safe_relative("a\\b"));
    assert!(!is_safe_relative("a\u{0}b"));
}

#[test]
fn test_is_crate_dir_name_rules() {
    for ok in ["example-crate", "a", "sample-app2"] {
        assert!(is_crate_dir_name(ok), "{ok} must be valid");
    }
    for bad in [
        "",
        "-x",
        "X",
        "has_underscore",
        "has/slash",
        "a..b",
        "toolongx".repeat(20).as_str(),
    ] {
        assert!(!is_crate_dir_name(bad), "{bad} must be invalid");
    }
}
