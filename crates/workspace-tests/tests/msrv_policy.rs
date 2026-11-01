#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Parses a Rust version string like "1.88", "1.88.0", or "1.89.0" into (major, minor, patch).
fn parse_rust_version(v: &str) -> Result<(u64, u64, u64), String> {
    let clean = v.trim().trim_start_matches('v');
    let parts: Vec<&str> = clean.split('.').collect();
    if parts.is_empty() || parts.len() > 3 {
        return Err(format!("Invalid version format: '{v}'"));
    }

    let major = parts[0]
        .parse::<u64>()
        .map_err(|e| format!("Invalid major version in '{v}': {e}"))?;
    let minor = if parts.len() > 1 {
        parts[1]
            .parse::<u64>()
            .map_err(|e| format!("Invalid minor version in '{v}': {e}"))?
    } else {
        0
    };
    let patch = if parts.len() > 2 {
        parts[2]
            .parse::<u64>()
            .map_err(|e| format!("Invalid patch version in '{v}': {e}"))?
    } else {
        0
    };

    Ok((major, minor, patch))
}

/// Evaluates if a package's declared MSRV is compatible with the workspace MSRV.
/// Fails closed on unparseable declared MSRVs.
fn check_package_msrv(
    pkg_name: &str,
    pkg_version: &str,
    declared_rv: Option<&str>,
    workspace_msrv: &str,
) -> Result<(), String> {
    let ws_version = parse_rust_version(workspace_msrv)
        .map_err(|e| format!("Invalid workspace MSRV '{workspace_msrv}': {e}"))?;

    if let Some(rv_str) = declared_rv {
        let pkg_version_parsed = parse_rust_version(rv_str).map_err(|_| {
            format!(
                "Package '{pkg_name}' v{pkg_version} declares unparseable rust-version '{rv_str}'"
            )
        })?;

        if pkg_version_parsed > ws_version {
            return Err(format!(
                "Package '{pkg_name}' v{pkg_version} requires rustc {rv_str}, which exceeds workspace MSRV {workspace_msrv}"
            ));
        }
    }

    Ok(())
}

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("Failed to locate workspace repo root")
        .to_path_buf()
}

#[test]
fn test_msrv_policy_synchronized() {
    let root = repo_root();

    // 1. rust-toolchain.toml
    let toolchain_content = fs::read_to_string(root.join("rust-toolchain.toml"))
        .expect("failed to read rust-toolchain.toml");
    let toolchain_channel = toolchain_content
        .lines()
        .find(|l| l.trim().starts_with("channel"))
        .and_then(|l| l.split('"').nth(1))
        .expect("failed to extract channel from rust-toolchain.toml");

    // 2. Cargo.toml [workspace.package] rust-version
    let cargo_content =
        fs::read_to_string(root.join("Cargo.toml")).expect("failed to read Cargo.toml");
    let cargo_rust_version = cargo_content
        .lines()
        .find(|l| l.trim().starts_with("rust-version"))
        .and_then(|l| l.split('"').nth(1))
        .expect("failed to extract rust-version from Cargo.toml");

    // 3. .clippy.toml msrv
    let clippy_content =
        fs::read_to_string(root.join(".clippy.toml")).expect("failed to read .clippy.toml");
    let clippy_msrv = clippy_content
        .lines()
        .find(|l| l.trim().starts_with("msrv"))
        .and_then(|l| l.split('"').nth(1))
        .expect("failed to extract msrv from .clippy.toml");

    // 4. .github/workflows/ci.yml msrv job toolchain
    let ci_content = fs::read_to_string(root.join(".github/workflows/ci.yml"))
        .expect("failed to read .github/workflows/ci.yml");
    let mut in_msrv_job = false;
    let mut ci_toolchain = None;
    for line in ci_content.lines() {
        if line.trim().starts_with("msrv:") {
            in_msrv_job = true;
            continue;
        }
        if in_msrv_job {
            if line.starts_with("  #")
                || (line.starts_with("  ") && !line.starts_with("    ") && line.ends_with(':'))
            {
                break;
            }
            if line.contains("toolchain:") {
                ci_toolchain = line.split('"').nth(1).map(String::from);
                break;
            }
        }
    }
    let ci_msrv = ci_toolchain.expect("failed to extract toolchain from ci.yml msrv job");

    assert_eq!(
        toolchain_channel, cargo_rust_version,
        "rust-toolchain.toml channel ({toolchain_channel}) != Cargo.toml rust-version ({cargo_rust_version})"
    );
    assert_eq!(
        toolchain_channel, clippy_msrv,
        "rust-toolchain.toml channel ({toolchain_channel}) != .clippy.toml msrv ({clippy_msrv})"
    );
    assert_eq!(
        toolchain_channel, ci_msrv,
        "rust-toolchain.toml channel ({toolchain_channel}) != ci.yml msrv toolchain ({ci_msrv})"
    );
}

#[test]
fn test_regression_msrv_evaluator() {
    let ws_msrv = "1.88";

    // Regression fixture for PR #468 exact failure class:
    // rusqlite 0.41.1 requires rustc 1.89.0
    let failure_result = check_package_msrv("rusqlite", "0.41.1", Some("1.89.0"), ws_msrv);
    assert!(
        failure_result.is_err(),
        "Expected 1.89.0 requirement to fail on 1.88 workspace"
    );
    let err = failure_result.unwrap_err();
    assert!(err.contains("rusqlite"), "Error must name crate: {err}");
    assert!(err.contains("0.41.1"), "Error must name version: {err}");
    assert!(
        err.contains("1.89.0"),
        "Error must name required rustc: {err}"
    );
    assert!(
        err.contains("1.88"),
        "Error must name workspace MSRV: {err}"
    );

    // Compatible version must pass
    let pass_exact = check_package_msrv("rusqlite", "0.40.2", Some("1.88.0"), ws_msrv);
    assert!(pass_exact.is_ok());

    let pass_older = check_package_msrv("anyhow", "1.0.104", Some("1.68"), ws_msrv);
    assert!(pass_older.is_ok());

    let pass_none = check_package_msrv("legacy-crate", "0.1.0", None, ws_msrv);
    assert!(pass_none.is_ok());

    // Fail closed on unparseable declared MSRV
    let fail_unparseable = check_package_msrv("bad-crate", "1.0.0", Some("not_a_version"), ws_msrv);
    assert!(fail_unparseable.is_err());
    let err_unparseable = fail_unparseable.unwrap_err();
    assert!(
        err_unparseable.contains("unparseable"),
        "Must fail closed on unparseable MSRV: {err_unparseable}"
    );
}

#[test]
fn test_resolved_dependencies_satisfy_msrv() {
    let root = repo_root();
    let cargo_content =
        fs::read_to_string(root.join("Cargo.toml")).expect("failed to read Cargo.toml");
    let ws_msrv = cargo_content
        .lines()
        .find(|l| l.trim().starts_with("rust-version"))
        .and_then(|l| l.split('"').nth(1))
        .expect("failed to extract rust-version from Cargo.toml");

    let output = Command::new("cargo")
        .current_dir(&root)
        .args(["metadata", "--format-version", "1", "--all-features"])
        .output()
        .expect("failed to run cargo metadata");

    assert!(output.status.success(), "cargo metadata failed");

    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("failed to parse cargo metadata JSON");

    let mut violations = Vec::new();
    if let Some(packages) = metadata["packages"].as_array() {
        for pkg in packages {
            let name = pkg["name"].as_str().unwrap_or("unknown");
            let version = pkg["version"].as_str().unwrap_or("unknown");
            let rust_version = pkg["rust_version"].as_str();

            if let Err(msg) = check_package_msrv(name, version, rust_version, ws_msrv) {
                violations.push(msg);
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Resolved dependencies exceed workspace MSRV ({ws_msrv}):\n{}",
        violations.join("\n")
    );
}
