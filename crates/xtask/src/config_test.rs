//! Fail-closed configuration tests: structural validation and corrupt-file handling
//! (kept out-of-line to respect the 500-LOC limit).
#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
use std::io::Write as _;

/// Writes `content` to a uniquely named file inside `dir` and returns its path.
fn config_in(dir: &tempfile::TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    let mut file = File::create(&path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    path
}

const MINIMAL_BODY: &str = r#""env_var_name":"XTASK_TIER","lint_thresholds":{"max_lines_per_file":500,"clippy_warnings_as_errors":true},"#;

#[test]
fn corrupt_json_fails_closed_with_invalid_config() {
    let dir = tempfile::tempdir().unwrap();
    let path = config_in(&dir, "corrupt.json", "{ not valid json ");
    let err = XtaskConfig::load_from_file(&path).unwrap_err();
    assert!(
        matches!(err, XtaskError::InvalidConfig { .. }),
        "expected InvalidConfig, got {err:?}"
    );
    assert!(
        err.to_string().contains("parse"),
        "error should mention parsing: {err}"
    );
}

#[test]
fn unknown_check_variant_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    let path = config_in(
        &dir,
        "unknown-check.json",
        &format!(
            "{{{MINIMAL_BODY}\"default_tier\":\"ci-smoke\",\"tiers\":{{\"ci-smoke\":{{\"checks\":[\"NotACheck\"]}}}}}}"
        ),
    );
    let err = XtaskConfig::load_from_file(&path).unwrap_err();
    assert!(
        matches!(err, XtaskError::InvalidConfig { .. }),
        "expected InvalidConfig, got {err:?}"
    );
    assert!(
        err.to_string().contains("NotACheck"),
        "serde must name the unknown variant: {err}"
    );
}

#[test]
fn directory_config_path_is_invalid() {
    let dir = tempfile::tempdir().unwrap();
    let err = XtaskConfig::load_from_file(dir.path()).unwrap_err();
    assert!(matches!(err, XtaskError::InvalidConfig { .. }), "{err:?}");
    assert!(err.to_string().contains("not a file"), "{err}");
}

#[test]
fn default_tier_must_name_a_defined_tier_after_alias_resolution() {
    let dir = tempfile::tempdir().unwrap();
    // Legacy alias "fast-pr" resolves to the built-in pull-request tier: valid.
    let aliased = config_in(
        &dir,
        "aliased.json",
        &format!("{{{MINIMAL_BODY}\"default_tier\":\"fast-pr\"}}"),
    );
    assert!(XtaskConfig::load_from_file(&aliased).is_ok());
    // An unknown default tier silently reverting to built-ins is the fail-open
    // this validation closes: it must be a hard error.
    let unknown = config_in(
        &dir,
        "unknown-tier.json",
        &format!(
            "{{{MINIMAL_BODY}\"default_tier\":\"nightly\",\"tiers\":{{\"pr\":{{\"checks\":[\"Fmt\"]}}}}}}"
        ),
    );
    let err = XtaskConfig::load_from_file(&unknown).unwrap_err();
    assert!(
        err.to_string().contains("nightly"),
        "error must name the unresolvable tier: {err}"
    );
}

#[test]
fn empty_tier_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = config_in(
        &dir,
        "empty-tier.json",
        &format!(
            "{{{MINIMAL_BODY}\"default_tier\":\"pr\",\"tiers\":{{\"pr\":{{\"checks\":[]}}}}}}"
        ),
    );
    let err = XtaskConfig::load_from_file(&path).unwrap_err();
    assert!(
        err.to_string().contains("no checks"),
        "empty tier must be rejected: {err}"
    );
}

#[test]
fn duplicate_checks_within_a_tier_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = config_in(
        &dir,
        "dup-tier.json",
        &format!(
            "{{{MINIMAL_BODY}\"default_tier\":\"pr\",\"tiers\":{{\"pr\":{{\"checks\":[\"Fmt\",\"Fmt\"]}}}}}}"
        ),
    );
    let err = XtaskConfig::load_from_file(&path).unwrap_err();
    assert!(
        err.to_string().contains("duplicate"),
        "duplicate checks must be rejected: {err}"
    );
}

#[test]
fn required_checks_must_reference_listed_checks() {
    let dir = tempfile::tempdir().unwrap();
    let path = config_in(
        &dir,
        "unlisted-required.json",
        &format!(
            "{{{MINIMAL_BODY}\"default_tier\":\"release\",\"tiers\":{{\"release\":{{\"checks\":[\"Fmt\"],\"required_checks\":[\"Audit\"]}}}}}}"
        ),
    );
    let err = XtaskConfig::load_from_file(&path).unwrap_err();
    assert!(
        err.to_string().contains("Audit"),
        "error must name the unlisted required check: {err}"
    );
}

#[test]
fn explicit_required_checks_are_loaded() {
    let dir = tempfile::tempdir().unwrap();
    let path = config_in(
        &dir,
        "explicit-required.json",
        &format!(
            "{{{MINIMAL_BODY}\"default_tier\":\"pr\",\"tiers\":{{\"pr\":{{\"checks\":[\"Audit\",\"Fmt\"],\"required_checks\":[\"Audit\"]}}}}}}"
        ),
    );
    let config = XtaskConfig::load_from_file(&path).unwrap();
    assert_eq!(
        config.tiers["pr"].required_checks,
        Some(vec![crate::quality::QualityCheck::Audit])
    );
}

#[test]
fn builtin_default_config_passes_validation() {
    XtaskConfig::default().validate().unwrap();
}

#[test]
fn missing_config_file_keeps_the_deliberate_defaults_fallback() {
    // Documents the intentional missing-file semantics: an absent config yields
    // the built-in defaults (equivalent to the shipped configuration); only a
    // *present-but-invalid* config is fatal.
    let result = XtaskConfig::load_from_file("definitely-not-here.json").unwrap();
    assert!(result.validate().is_ok());
    assert_eq!(result.tiers.len(), 4);
}
