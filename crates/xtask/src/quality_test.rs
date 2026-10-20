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

#[test]
fn test_when_changed_selection_manifest_edits() {
    let config = XtaskConfig::default();

    // 1. Member manifest inside crates/ without Cargo.lock change
    let cp = ChangedPaths::classify(&["crates/sample-app/Cargo.toml"]);
    let expected_checks = [
        QualityCheck::Fmt,
        QualityCheck::Clippy,
        QualityCheck::Build,
        QualityCheck::Test,
        QualityCheck::DocTest,
        QualityCheck::Audit,
        QualityCheck::Deny,
        QualityCheck::Machete,
        QualityCheck::Msrv,
    ];
    for check in expected_checks {
        let (selected, reason) = check_matches_when_changed(check, &config, &cp);
        assert!(
            selected,
            "Check {} should be selected for crates/sample-app/Cargo.toml edit. Reason: {reason}",
            check.name()
        );
    }

    // 2. Member manifest outside crates/ (e.g. examples/hello_world/Cargo.toml)
    let cp_outside = ChangedPaths::classify(&["examples/hello_world/Cargo.toml"]);
    for check in expected_checks {
        let (selected, reason) = check_matches_when_changed(check, &config, &cp_outside);
        assert!(
            selected,
            "Check {} should be selected for examples/hello_world/Cargo.toml edit. Reason: {reason}",
            check.name()
        );
    }

    // 3. Deletion of a Rust source file or manifest
    let cp_deleted = ChangedPaths::classify(&["crates/sample-app/src/old.rs"]);
    let (selected_fmt, _) = check_matches_when_changed(QualityCheck::Fmt, &config, &cp_deleted);
    let (selected_test, _) = check_matches_when_changed(QualityCheck::Test, &config, &cp_deleted);
    assert!(selected_fmt);
    assert!(selected_test);

    let cp_deleted_manifest = ChangedPaths::classify(&["crates/old-crate/Cargo.toml"]);
    for check in expected_checks {
        let (selected, reason) = check_matches_when_changed(check, &config, &cp_deleted_manifest);
        assert!(
            selected,
            "Check {} should be selected for deleted manifest. Reason: {reason}",
            check.name()
        );
    }

    // 4. Docs-only edit should NOT select Rust compilation/test checks
    let cp_docs = ChangedPaths::classify(&["README.md"]);
    let (selected_test_docs, _) = check_matches_when_changed(QualityCheck::Test, &config, &cp_docs);
    let (selected_md, _) = check_matches_when_changed(QualityCheck::MarkdownLint, &config, &cp_docs);
    assert!(!selected_test_docs);
    assert!(selected_md);
}

#[test]
fn test_has_executed_tests_detection() {
    let output_with_tests = "\
running 2 tests
test tests::test_one ... ok
test tests::test_two ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s";
    assert!(has_executed_tests(output_with_tests));

    let output_zero_tests = "\
running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s";
    assert!(!has_executed_tests(output_zero_tests));
}
