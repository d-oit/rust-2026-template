//! Unit tests for quality planning and execution.
#![allow(clippy::unwrap_used)]

use super::*;

#[test]
fn test_plan_checks_fast_pr() {
    let config = XtaskConfig::default();
    let checks = plan_checks(&config, Some("fast-pr"), None, None).unwrap();
    assert!(checks.contains(&QualityCheck::LocLimits));
    assert!(checks.contains(&QualityCheck::Fmt));
    assert!(checks.contains(&QualityCheck::Test));
    assert!(!checks.contains(&QualityCheck::ShellCheck));
}

#[test]
fn test_plan_checks_only() {
    let config = XtaskConfig::default();
    let checks = plan_checks(&config, Some("fast-pr"), Some("fmt,clippy"), None).unwrap();
    assert_eq!(checks.len(), 2);
    assert!(checks.contains(&QualityCheck::Fmt));
    assert!(checks.contains(&QualityCheck::Clippy));
}

#[test]
fn test_plan_checks_canonical_tiers() {
    let config = XtaskConfig::default();
    // pull-request is the fast correctness tier.
    let pr = plan_checks(&config, Some("pull-request"), None, None).unwrap();
    assert!(pr.contains(&QualityCheck::Test));
    assert!(!pr.contains(&QualityCheck::Deny));
    // GOAP guardrail (ADR 0005) must run on every PR, not just post-merge.
    assert!(pr.contains(&QualityCheck::WorkflowValidation));
    // protected-branch is the deep merge gate.
    let merge = plan_checks(&config, Some("protected-branch"), None, None).unwrap();
    assert!(merge.contains(&QualityCheck::Deny));
    assert!(merge.contains(&QualityCheck::Audit));
    // scheduled carries the expensive eval/roast checks, not the PR-tier trivia.
    let scheduled = plan_checks(&config, Some("scheduled"), None, None).unwrap();
    assert!(scheduled.contains(&QualityCheck::RoastScorer));
    // release is the pre-release security + build gate.
    let release = plan_checks(&config, Some("release"), None, None).unwrap();
    assert!(release.contains(&QualityCheck::Audit));
}

#[test]
fn test_plan_checks_legacy_aliases() {
    let config = XtaskConfig::default();
    let fast = plan_checks(&config, Some("fast-pr"), None, None).unwrap();
    let pr = plan_checks(&config, Some("pull-request"), None, None).unwrap();
    assert_eq!(fast, pr);
    let full = plan_checks(&config, Some("full-gate"), None, None).unwrap();
    let all = plan_checks(&config, Some("all"), None, None).unwrap();
    let merge = plan_checks(&config, Some("protected-branch"), None, None).unwrap();
    assert_eq!(full, merge);
    assert_eq!(all, merge);
}

#[test]
fn test_plan_checks_unknown_tier_is_error() {
    let config = XtaskConfig::default();
    let err = plan_checks(&config, Some("no-such-tier"), None, None).unwrap_err();
    assert!(err.to_string().contains("no-such-tier"));
}

#[test]
fn test_plan_checks_custom_tier_from_config() {
    let mut config = XtaskConfig::default();
    config.tiers.insert(
        "ci-smoke".to_string(),
        crate::config::TierDef {
            checks: vec![QualityCheck::Fmt, QualityCheck::Clippy],
            required_checks: None,
        },
    );
    let checks = plan_checks(&config, Some("ci-smoke"), None, None).unwrap();
    assert_eq!(checks, vec![QualityCheck::Fmt, QualityCheck::Clippy]);
}

#[test]
fn test_plan_checks_detailed_when_changed_glob_matching() {
    let config = XtaskConfig::default();
    // HEAD SHA or valid git commit
    let plan = plan_checks_detailed(&config, Some("protected-branch"), None, Some("HEAD")).unwrap();
    assert!(!plan.full_tier_checks.is_empty());
    assert!(!plan.details.is_empty());
}

#[test]
fn test_plan_checks_detailed_unreadable_git_fallback() {
    let config = XtaskConfig::default();
    // Invalid SHA triggers git error -> fail-closed fallback
    let plan = plan_checks_detailed(
        &config,
        Some("protected-branch"),
        None,
        Some("nonexistent_invalid_sha_12345"),
    )
    .unwrap();
    assert!(plan.fallback_used);
    assert_eq!(plan.selected_checks, plan.full_tier_checks);
    assert!(
        plan.details
            .iter()
            .all(|d| d.selected && d.reason.contains("fail-closed fallback active"))
    );
}
