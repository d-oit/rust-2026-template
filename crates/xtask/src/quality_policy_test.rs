//! Required-tool policy tests: requiredness resolution, tool-presence decision, and
//! fail-closed skip behavior (kept out-of-line to respect the 500-LOC limit).
#![allow(clippy::unwrap_used, clippy::panic)]
use super::{
    CheckTool, DEFAULT_REQUIRED_CHECKS, ToolRequirement, absent_outcome, check_tool, is_required,
    required_checks, set_active_tier, tool_present_with,
};
use crate::config::{TierDef, XtaskConfig, XtaskError};
use crate::quality::{QualityCheck as Q, plan_checks};
use std::collections::BTreeSet;

/// Every check variant, used to build fully-populated tier definitions.
const ALL_CHECKS: [Q; 21] = [
    Q::LocLimits,
    Q::SkillValidation,
    Q::AdrCompliance,
    Q::Fmt,
    Q::Clippy,
    Q::Build,
    Q::Test,
    Q::DocTest,
    Q::Audit,
    Q::Deny,
    Q::Machete,
    Q::Msrv,
    Q::ShellCheck,
    Q::MarkdownLint,
    Q::PrivacyCheck,
    Q::SecretScan,
    Q::WorkflowValidation,
    Q::SkillEvals,
    Q::LlmContext,
    Q::CiStatusArtifact,
    Q::RoastScorer,
];

fn tier_def(checks: &[Q]) -> TierDef {
    TierDef {
        checks: checks.to_vec(),
        required_checks: None,
    }
}

#[test]
fn builtin_policy_requires_security_checks_on_protected_tiers() {
    for tier in ["protected-branch", "release"] {
        let required = required_checks(tier, &tier_def(&ALL_CHECKS));
        for check in DEFAULT_REQUIRED_CHECKS {
            assert!(
                required.contains(&check),
                "tier '{tier}' must require {}",
                check.name()
            );
        }
        assert_eq!(required.len(), DEFAULT_REQUIRED_CHECKS.len());
    }
}

#[test]
fn builtin_policy_keeps_other_tiers_advisory() {
    for tier in ["pull-request", "scheduled", "adopters-custom-tier"] {
        let required = required_checks(tier, &tier_def(&ALL_CHECKS));
        assert!(
            required.is_empty(),
            "tier '{tier}' must stay advisory by default"
        );
    }
}

#[test]
fn explicit_required_checks_override_the_builtin_policy() {
    let mut def = tier_def(&ALL_CHECKS);
    def.required_checks = Some(vec![Q::Audit]);
    assert_eq!(
        required_checks("pull-request", &def),
        BTreeSet::from([Q::Audit])
    );
    // An explicit empty list is a deliberate opt-out, even on protected tiers.
    def.required_checks = Some(Vec::new());
    assert!(required_checks("protected-branch", &def).is_empty());
}

#[test]
fn table_every_required_check_either_maps_to_a_tool_or_is_tool_free() {
    for check in DEFAULT_REQUIRED_CHECKS {
        match check_tool(check) {
            Some(CheckTool {
                tool_name,
                guidance,
                ..
            }) => {
                assert!(!tool_name.is_empty());
                assert!(
                    !guidance.is_empty(),
                    "{tool_name} must carry install guidance"
                );
            }
            // SecretScan runs in-process (regex scan) and can never be skipped
            // for lack of a tool, so its requiredness is vacuously satisfied.
            None => assert_eq!(check, Q::SecretScan),
        }
    }
}

#[test]
fn table_required_check_with_absent_tool_is_a_missing_tool_error() {
    for check in DEFAULT_REQUIRED_CHECKS {
        let Some(tool) = check_tool(check) else {
            continue; // tool-free checks (SecretScan) cannot have a missing tool
        };
        let err = absent_outcome(&tool, true, "protected-branch").unwrap_err();
        assert!(
            matches!(err, XtaskError::MissingTool { .. }),
            "expected MissingTool for {}, got {err:?}",
            check.name()
        );
        if let XtaskError::MissingTool {
            tool_name,
            guidance,
        } = err
        {
            assert_eq!(tool_name, tool.tool_name);
            assert!(
                guidance.contains("required by tier 'protected-branch'"),
                "guidance must name the tier: {guidance}"
            );
            assert!(
                guidance.contains(tool.guidance),
                "guidance must carry the install hint: {guidance}"
            );
        }
    }
}

#[test]
fn table_optional_check_with_absent_tool_keeps_the_skip_behavior() {
    for check in DEFAULT_REQUIRED_CHECKS {
        let Some(tool) = check_tool(check) else {
            continue;
        };
        absent_outcome(&tool, false, "pull-request").unwrap();
    }
}

#[test]
fn table_present_tools_resolve_through_injected_probes() {
    for check in DEFAULT_REQUIRED_CHECKS {
        let Some(tool) = check_tool(check) else {
            continue;
        };
        let present = tool_present_with(&tool.requirement, |_t, _args| true, |_path| true);
        assert!(present, "{} must resolve when present", tool.tool_name);
    }
}

#[test]
fn table_absent_tools_do_not_resolve_through_injected_probes() {
    for check in DEFAULT_REQUIRED_CHECKS {
        let Some(tool) = check_tool(check) else {
            continue;
        };
        let present = tool_present_with(&tool.requirement, |_t, _args| false, |_path| false);
        assert!(!present, "{} must not resolve when absent", tool.tool_name);
    }
}

#[test]
fn probe_arguments_match_the_documented_tool_invocations() {
    let cases = [
        (
            Q::Audit,
            "cargo-audit",
            ToolRequirement::Binary {
                tool: "cargo",
                version_args: &["audit", "--version"],
            },
        ),
        (
            Q::Deny,
            "cargo-deny",
            ToolRequirement::Binary {
                tool: "cargo",
                version_args: &["deny", "--version"],
            },
        ),
        (
            Q::Machete,
            "cargo-machete",
            ToolRequirement::Binary {
                tool: "cargo-machete",
                version_args: &["--version"],
            },
        ),
        (
            Q::ShellCheck,
            "shellcheck",
            ToolRequirement::Binary {
                tool: "shellcheck",
                version_args: &["--version"],
            },
        ),
        (
            Q::MarkdownLint,
            "markdownlint-cli2",
            ToolRequirement::Binary {
                tool: "markdownlint-cli2",
                version_args: &["--version"],
            },
        ),
        (
            Q::Msrv,
            "scripts/audit-msrv.sh",
            ToolRequirement::Script {
                path: "scripts/audit-msrv.sh",
            },
        ),
        (
            Q::WorkflowValidation,
            "scripts/validate-workflows.sh",
            ToolRequirement::Script {
                path: "scripts/validate-workflows.sh",
            },
        ),
    ];
    for (check, tool_name, requirement) in cases {
        let tool = check_tool(check).unwrap();
        assert_eq!(tool.tool_name, tool_name);
        assert_eq!(tool.requirement, requirement);
    }
}

#[test]
fn requiredness_follows_the_published_tier_context() {
    let config = XtaskConfig::default();
    set_active_tier("protected-branch");
    assert!(is_required(&config, Q::Audit));
    assert!(is_required(&config, Q::ShellCheck));
    set_active_tier("pull-request");
    assert!(!is_required(&config, Q::Audit));
    assert!(!is_required(&config, Q::SecretScan));
}

#[test]
fn plan_checks_publishes_the_tier_context_for_run_time_policy() {
    let config = XtaskConfig::default();
    plan_checks(&config, Some("fast-pr"), None, None).unwrap();
    assert!(
        !is_required(&config, Q::Audit),
        "pull-request tier stays advisory"
    );
    plan_checks(&config, Some("full-gate"), None, None).unwrap();
    assert!(
        is_required(&config, Q::Audit),
        "protected-branch tier fails closed"
    );
    assert!(is_required(&config, Q::WorkflowValidation));
}

#[test]
fn config_can_require_checks_on_any_tier() {
    let mut config = XtaskConfig::default();
    config.tiers.insert(
        "pr-with-audit".to_string(),
        TierDef {
            checks: vec![Q::Fmt, Q::Audit],
            required_checks: Some(vec![Q::Audit]),
        },
    );
    set_active_tier("pr-with-audit");
    assert!(is_required(&config, Q::Audit));
    assert!(!is_required(&config, Q::Fmt));
}
