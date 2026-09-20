#![allow(clippy::unwrap_used)]
use super::*;
use std::path::PathBuf;

fn sample_telemetry() -> CiTelemetry {
    CiTelemetry {
        schema_version: SCHEMA_VERSION,
        timestamp: "2026-08-09T00:00:00Z".to_string(),
        tier: "pull-request".to_string(),
        plan_source: "config/xtask.json".to_string(),
        scope: TelemetryScope {
            mode: "affected-packages".to_string(),
            packages: vec!["xtask".to_string()],
            fallback_used: false,
        },
        stages: vec![
            TelemetryStage {
                id: "rust-format".to_string(),
                status: "passed".to_string(),
                duration_ms: 12,
                cache: "not-applicable".to_string(),
                skipped_reason: None,
            },
            TelemetryStage {
                id: "rust-tests".to_string(),
                status: "skipped".to_string(),
                duration_ms: 0,
                cache: "not-applicable".to_string(),
                skipped_reason: Some("not affected by changed paths".to_string()),
            },
        ],
        toolchain: ToolchainInfo {
            rustc: "rustc 1.88.0".to_string(),
            cargo: "cargo 1.88.0".to_string(),
            nextest: "cargo-nextest 0.9".to_string(),
        },
        fingerprint: Some(EvidenceFingerprint {
            head_commit: "abc1234567890def".to_string(),
            worktree_hash: "1234567890abcdef".to_string(),
            tier: "pull-request".to_string(),
            policy_hash: "fedcba0987654321".to_string(),
        }),
    }
}

#[test]
fn test_serializes_with_schema_version() {
    let json = serde_json::to_value(sample_telemetry()).unwrap();
    assert_eq!(json["schema_version"], 2);
    assert_eq!(json["scope"]["mode"], "affected-packages");
    assert_eq!(json["stages"][1]["status"], "skipped");
    // No secrets/source fields are emitted by the struct.
    let keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert!(
        !keys
            .iter()
            .any(|k| k.to_lowercase().contains("token") || k.to_lowercase().contains("secret"))
    );
}

#[test]
fn test_emit_writes_artifact_and_summary() {
    let dir = tempfile::tempdir().unwrap();
    let config = TelemetryConfig {
        enabled: true,
        detail: "full".to_string(),
        retention_days: 7,
        summary_path: dir
            .path()
            .join("quality-summary.md")
            .to_string_lossy()
            .into_owned(),
        artifact_path: dir
            .path()
            .join("quality-run.json")
            .to_string_lossy()
            .into_owned(),
        budgets: TelemetryBudgets {
            max_stage_duration_ms: 600_000,
        },
    };
    sample_telemetry().emit(&config).unwrap();
    let artifact: CiTelemetry =
        serde_json::from_str(&std::fs::read_to_string(&config.artifact_path).unwrap()).unwrap();
    assert_eq!(artifact.schema_version, SCHEMA_VERSION);
    assert_eq!(artifact.stages.len(), 2);
    let summary = std::fs::read_to_string(&config.summary_path).unwrap();
    assert!(summary.contains("pull-request"));
    assert!(summary.contains("rust-format"));
}

#[test]
fn test_config_load_disabled_skips_emit() {
    let dir = tempfile::tempdir().unwrap();
    let config = TelemetryConfig {
        enabled: false,
        detail: "full".to_string(),
        retention_days: 7,
        summary_path: dir.path().join("s.md").to_string_lossy().into_owned(),
        artifact_path: dir.path().join("a.json").to_string_lossy().into_owned(),
        budgets: TelemetryBudgets {
            max_stage_duration_ms: 600_000,
        },
    };
    sample_telemetry().emit(&config).unwrap();
    assert!(!PathBuf::from(&config.artifact_path).exists());
}

#[test]
fn test_summary_marks_budget_exceeded() {
    let mut t = sample_telemetry();
    t.stages[0].duration_ms = 999_999;
    let md = t.summary_markdown(&TelemetryConfig::default());
    assert!(md.contains("Budget exceeded"));
    assert!(md.contains("rust-format"));
}

#[test]
fn test_stage_id_kebab() {
    assert_eq!(stage_id("Rust Format"), "rust-format");
    assert_eq!(
        stage_id("CI Status Artifact Check"),
        "ci-status-artifact-check"
    );
}

#[test]
fn test_freshness_green_when_matching() {
    let t = sample_telemetry();
    let fp = t.fingerprint.clone().unwrap();
    assert_eq!(t.check_freshness(&fp), EvidenceStatus::Green);
}

#[test]
fn test_freshness_red_when_failed_stage() {
    let mut t = sample_telemetry();
    t.stages[0].status = "failed".to_string();
    let fp = t.fingerprint.clone().unwrap();
    assert_eq!(t.check_freshness(&fp), EvidenceStatus::Red);
}

#[test]
fn test_freshness_stale_when_fingerprint_mismatch() {
    let t = sample_telemetry();
    let mut fp = t.fingerprint.clone().unwrap();
    fp.worktree_hash = "different_hash".to_string();
    assert_eq!(t.check_freshness(&fp), EvidenceStatus::Stale);

    let mut fp2 = t.fingerprint.clone().unwrap();
    fp2.policy_hash = "different_policy".to_string();
    assert_eq!(t.check_freshness(&fp2), EvidenceStatus::Stale);
}

#[test]
fn test_freshness_stale_when_no_fingerprint_v1() {
    let mut t = sample_telemetry();
    t.schema_version = 1;
    t.fingerprint = None;
    let fp = EvidenceFingerprint {
        head_commit: "abc1234567890def".to_string(),
        worktree_hash: "1234567890abcdef".to_string(),
        tier: "pull-request".to_string(),
        policy_hash: "fedcba0987654321".to_string(),
    };
    assert_eq!(t.check_freshness(&fp), EvidenceStatus::Stale);
}

#[test]
fn test_compute_fingerprint_returns_valid_hashes() {
    let fp = compute_fingerprint("pull-request");
    assert_eq!(fp.tier, "pull-request");
    assert!(!fp.head_commit.is_empty());
    assert_eq!(fp.worktree_hash.len(), 16);
    assert_eq!(fp.policy_hash.len(), 16);
}

/// The stage table used to abut the toolchain bullet, which the repo's own
/// markdownlint gate (MD058) rejects on the next run.
#[test]
fn test_summary_separates_stage_table_from_toolchain_bullet() {
    let md = sample_telemetry().summary_markdown(&TelemetryConfig::default());
    assert!(
        md.contains("\n\n- **Toolchain:**"),
        "stage table must be separated from the toolchain bullet by a blank line"
    );
    assert!(
        md.lines().all(|line| line == line.trim_end()),
        "generated summary must not contain trailing whitespace"
    );
}
