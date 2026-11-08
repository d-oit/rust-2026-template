#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    missing_docs,
    reason = "test modules assert on known-good fixtures, so panicking macros are the intended failure signal"
)]

//! MSRV policy contract tests.
//!
//! These tests must hold for any repository derived from this template, not only
//! for the reference workspace: every configuration that declares an MSRV is
//! compared as a parsed version, and configuration a downstream project
//! legitimately omits (or replaces with a channel alias) is skipped instead of
//! failing the suite.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A parsed Rust version as `(major, minor, patch)`.
type RustVersion = (u64, u64, u64);

/// Parses "1.88", "1.88.0", "v1.88.0" into a comparable version.
/// Returns `None` for anything that is not a plain numeric version, so channel
/// aliases ("stable", "beta", "nightly") are never mistaken for an MSRV.
fn parse_rust_version(raw: &str) -> Option<RustVersion> {
    let clean = raw.trim().trim_start_matches('v');
    let mut parts = clean.split('.').map(str::trim);

    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = match parts.next() {
        Some(part) => part.parse::<u64>().ok()?,
        None => 0,
    };
    let patch = match parts.next() {
        Some(part) => part.parse::<u64>().ok()?,
        None => 0,
    };
    if parts.next().is_some() {
        return None;
    }

    Some((major, minor, patch))
}

fn format_version(version: RustVersion) -> String {
    format!("{}.{}.{}", version.0, version.1, version.2)
}

/// Reads `key = "value"` from a TOML document. `section` is `None` for
/// top-level keys, otherwise the section name without brackets (for example
/// `"workspace.package"`).
fn toml_value(text: &str, section: Option<&str>, key: &str) -> Option<String> {
    let mut current: Option<&str> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix('[') {
            current = Some(rest.split(']').next().unwrap_or("").trim());
            continue;
        }
        if current != section {
            continue;
        }
        let Some(after_key) = trimmed.strip_prefix(key) else {
            continue;
        };
        let Some(after_eq) = after_key.trim_start().strip_prefix('=') else {
            continue;
        };
        let value = after_eq.trim();
        if let Some(inner) = value.strip_prefix('"') {
            return inner.split('"').next().map(str::to_string);
        }
        return Some(value.split('#').next().unwrap_or("").trim().to_string());
    }

    None
}

/// Locates the workspace root by walking up from this crate until a `Cargo.toml`
/// declaring `[workspace]` is found, so the layout of the consumer project is
/// not assumed.
fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if let Ok(text) = fs::read_to_string(dir.join("Cargo.toml"))
            && text
                .lines()
                .any(|line| line.trim() == "[workspace]" || line.trim() == "[workspace] ")
        {
            return dir;
        }
        assert!(
            dir.pop(),
            "no Cargo.toml declaring [workspace] above {}",
            env!("CARGO_MANIFEST_DIR")
        );
    }
}

/// Declared workspace MSRV: `[workspace.package]` for a workspace, `[package]`
/// for a single-crate project. `None` means the project declares no MSRV, so
/// there is no policy to enforce.
fn declared_msrv(root: &Path) -> Option<(RustVersion, String)> {
    let text = fs::read_to_string(root.join("Cargo.toml")).expect("root Cargo.toml is readable");
    let raw = toml_value(&text, Some("workspace.package"), "rust-version")
        .or_else(|| toml_value(&text, Some("package"), "rust-version"))?;
    parse_rust_version(&raw).map(|version| (version, raw))
}

fn assert_matches_workspace(
    source: &str,
    raw: &str,
    workspace: RustVersion,
    problems: &mut Vec<String>,
) {
    // A channel alias such as "stable" does not express an MSRV; only concrete
    // versions are compared.
    if let Some(version) = parse_rust_version(raw)
        && version != workspace
    {
        problems.push(format!(
            "{source} declares {raw}, but the workspace rust-version is {}",
            format_version(workspace)
        ));
    }
}

/// Returns `true` if a package's declared MSRV is compatible with the workspace
/// MSRV. Unparseable declarations fail closed.
fn check_package_msrv(
    pkg_name: &str,
    pkg_version: &str,
    declared: Option<&str>,
    workspace: RustVersion,
) -> Result<(), String> {
    let Some(raw) = declared else {
        return Ok(());
    };

    let Some(version) = parse_rust_version(raw) else {
        return Err(format!(
            "Package '{pkg_name}' v{pkg_version} declares unparseable rust-version '{raw}'"
        ));
    };

    if version > workspace {
        return Err(format!(
            "Package '{pkg_name}' v{pkg_version} requires rustc {raw}, which exceeds workspace MSRV {}",
            format_version(workspace)
        ));
    }

    Ok(())
}

/// Extracts the `toolchain:` input of a named job from a GitHub Actions workflow.
/// Returns `None` when the workflow, the job, or the input is absent — a job that
/// relies on `rust-toolchain.toml` declares no version to compare.
fn ci_job_toolchain(workflow: &str, job: &str) -> Option<String> {
    let header = format!("  {job}:");
    let mut lines = workflow.lines().skip_while(|line| *line != header);

    lines.next()?;

    for line in lines {
        let is_block_line = line.starts_with("    ") || line.trim().is_empty();
        if !is_block_line {
            // Next job (or end of the jobs mapping).
            return None;
        }
        if let Some(rest) = line.trim().strip_prefix("toolchain:") {
            let value = rest.trim();
            if let Some(inner) = value.strip_prefix('"') {
                return inner.split('"').next().map(str::to_string);
            }
            return Some(value.split('#').next().unwrap_or("").trim().to_string());
        }
    }

    None
}

#[test]
fn test_msrv_policy_synchronized() {
    let root = workspace_root();
    let Some((workspace, _raw)) = declared_msrv(&root) else {
        // No MSRV declared: nothing to synchronize.
        return;
    };

    let mut problems = Vec::new();

    if let Ok(toolchain) = fs::read_to_string(root.join("rust-toolchain.toml"))
        && let Some(channel) = toml_value(&toolchain, Some("toolchain"), "channel")
    {
        assert_matches_workspace(
            "rust-toolchain.toml channel",
            &channel,
            workspace,
            &mut problems,
        );
    }

    if let Ok(clippy) = fs::read_to_string(root.join(".clippy.toml"))
        && let Some(msrv) = toml_value(&clippy, None, "msrv")
    {
        assert_matches_workspace(".clippy.toml msrv", &msrv, workspace, &mut problems);
    }

    if let Ok(workflow) = fs::read_to_string(root.join(".github/workflows/ci.yml"))
        && let Some(toolchain) = ci_job_toolchain(&workflow, "msrv")
    {
        assert_matches_workspace(
            "ci.yml msrv job toolchain",
            &toolchain,
            workspace,
            &mut problems,
        );
    }

    assert!(
        problems.is_empty(),
        "MSRV policy drift detected (workspace rust-version {}):\n{}",
        format_version(workspace),
        problems.join("\n")
    );
}

#[test]
fn test_regression_msrv_evaluator() {
    let workspace = (1, 88, 0);

    // Exact failure class of PR #468: rusqlite 0.41.1 requires rustc 1.89.0.
    let failure = check_package_msrv("rusqlite", "0.41.1", Some("1.89.0"), workspace);
    let err = failure.expect_err("1.89.0 must not satisfy a 1.88 workspace");
    assert!(err.contains("rusqlite"), "error must name the crate: {err}");
    assert!(err.contains("0.41.1"), "error must name the version: {err}");
    assert!(
        err.contains("1.89.0"),
        "error must name the required rustc: {err}"
    );
    assert!(
        err.contains("1.88.0"),
        "error must name the workspace MSRV: {err}"
    );

    // Compatible declarations pass, including short and patch-qualified forms.
    assert!(check_package_msrv("rusqlite", "0.40.2", Some("1.88.0"), workspace).is_ok());
    assert!(check_package_msrv("short-form", "1.0.0", Some("1.88"), workspace).is_ok());
    assert!(check_package_msrv("anyhow", "1.0.104", Some("1.68"), workspace).is_ok());
    assert!(check_package_msrv("undeclared", "0.1.0", None, workspace).is_ok());

    // Fail closed on a declaration that cannot be evaluated.
    let unparseable = check_package_msrv("bad-crate", "1.0.0", Some("not-a-version"), workspace)
        .expect_err("unparseable rust-version must fail closed");
    assert!(
        unparseable.contains("unparseable"),
        "unexpected error: {unparseable}"
    );
}

#[test]
fn test_portability_of_config_parsing() {
    // Short and patch-qualified spellings compare equal.
    assert_eq!(parse_rust_version("1.88"), parse_rust_version("1.88.0"));
    assert_eq!(parse_rust_version("v1.88.0"), Some((1, 88, 0)));
    // Channel aliases are not versions.
    assert_eq!(parse_rust_version("stable"), None);
    assert_eq!(parse_rust_version("nightly"), None);

    // Comments and unrelated sections must not shadow the real value.
    let cargo = "[workspace]\nmembers = []\n\n[workspace.package]\n# rust-version = \"9.9\"\nedition = \"2024\"\nrust-version = \"1.88\"\n";
    assert_eq!(
        toml_value(cargo, Some("workspace.package"), "rust-version").as_deref(),
        Some("1.88")
    );
    assert_eq!(toml_value(cargo, Some("package"), "rust-version"), None);

    // A job without a toolchain input declares nothing to compare.
    let without_input = "jobs:\n  msrv:\n    name: MSRV Check\n    runs-on: ubuntu-latest\n  next:\n    runs-on: ubuntu-latest\n";
    assert_eq!(ci_job_toolchain(without_input, "msrv"), None);

    // The toolchain input is read from the right job only.
    let with_input = "jobs:\n  build:\n    steps:\n      - with:\n          toolchain: \"1.70\"\n  msrv:\n    steps:\n      - uses: dtolnay/rust-toolchain\n        with:\n          toolchain: \"1.88\"\n";
    assert_eq!(
        ci_job_toolchain(with_input, "msrv").as_deref(),
        Some("1.88")
    );
    assert_eq!(ci_job_toolchain(with_input, "absent"), None);
}

#[test]
fn test_resolved_dependencies_satisfy_msrv() {
    let root = workspace_root();
    let Some((workspace, raw_workspace)) = declared_msrv(&root) else {
        return;
    };

    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = Command::new(cargo)
        .current_dir(&root)
        .args(["metadata", "--format-version", "1", "--all-features"])
        .output()
        .expect("failed to run cargo metadata");

    assert!(
        output.status.success(),
        "cargo metadata failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("cargo metadata is valid JSON");

    let mut violations = Vec::new();
    for pkg in metadata["packages"].as_array().into_iter().flatten() {
        let name = pkg["name"].as_str().unwrap_or("unknown");
        let version = pkg["version"].as_str().unwrap_or("unknown");
        if let Err(message) =
            check_package_msrv(name, version, pkg["rust_version"].as_str(), workspace)
        {
            violations.push(message);
        }
    }

    assert!(
        violations.is_empty(),
        "resolved dependencies exceed the workspace MSRV {raw_workspace}:\n{}",
        violations.join("\n")
    );
}
