#!/usr/bin/env bash
# tests/workflow_maintenance_contract_test.sh
# Static contract tests for the CI maintenance fast path, the PR Merge Policy
# Gate, and the DORA report automation.
#
# These pin the parts of the design that a YAML edit could silently break:
# required check names, least-privilege permissions, the absence of
# `[skip ci]`, and the presence of the branch-refresh and auto-merge steps.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

CI=".github/workflows/ci.yml"
DORA=".github/workflows/dora-report.yml"
POLICY=".github/workflows/pr-merge-policy.yml"
DEPENDABOT=".github/workflows/dependabot-auto-merge.yml"

python3 - "$CI" "$DORA" "$POLICY" "$DEPENDABOT" <<'PY'
import re
import subprocess
import sys
from pathlib import Path

import yaml

ci_path, dora_path, policy_path, dependabot_path = (Path(arg) for arg in sys.argv[1:5])
ci_text = ci_path.read_text(encoding="utf-8")
dora_text = dora_path.read_text(encoding="utf-8")
policy_text = policy_path.read_text(encoding="utf-8")
dependabot_text = dependabot_path.read_text(encoding="utf-8")

ci = yaml.safe_load(ci_text)
dora = yaml.safe_load(dora_text)
policy = yaml.safe_load(policy_text)
dependabot = yaml.safe_load(dependabot_text)

failures = []


def check(condition, message):
    if not condition:
        failures.append(message)


# ── Actions lockfile verification is read-only and covers every workflow ────
lock_path = Path(".github/workflows/actions.lock")
lock_workflow_path = Path(".github/workflows/verify-actions-lock.yml")
check(lock_path.is_file(), "the generated GitHub Actions lockfile must be committed")
check(lock_workflow_path.is_file(), "the read-only lockfile verification workflow must exist")
if lock_path.is_file():
    diff_attr = subprocess.run(
        ["git", "check-attr", "diff", "--", lock_path.as_posix()],
        capture_output=True,
        check=True,
        text=True,
    ).stdout.strip()
    check(
        diff_attr.endswith(": set"),
        "actions.lock must stay diffable so dependency changes are reviewable",
    )

if lock_path.is_file() and lock_workflow_path.is_file():
    lock = yaml.safe_load(lock_path.read_text(encoding="utf-8")) or {}
    lock_workflow = yaml.safe_load(lock_workflow_path.read_text(encoding="utf-8")) or {}
    locked_workflows = lock.get("workflows") or {}
    locked_dependencies = lock.get("dependencies") or {}
    locked_dependency_names = {name.lower() for name in locked_dependencies}
    check(lock.get("version") == "v0.0.3", "actions.lock must use the generated lockfile format")
    check(".github/workflows/verify-actions-lock.yml" in locked_workflows,
          "the verifier workflow must itself be onboarded in actions.lock")
    check("d-oit/rust-2026-template" not in lock_path.read_text(encoding="utf-8"),
          "actions.lock must not contain repository-specific references")

    for workflow_path in sorted(Path(".github/workflows").glob("*.yml")):
        relative_path = workflow_path.as_posix()
        workflow = yaml.safe_load(workflow_path.read_text(encoding="utf-8")) or {}
        locked_refs = {name.lower() for name in locked_workflows.get(relative_path) or []}
        for job in (workflow.get("jobs") or {}).values():
            for step in job.get("steps") or []:
                uses = step.get("uses")
                if not uses or uses.startswith(("./", "$/")):
                    continue
                check(uses.lower() in locked_refs,
                      f"{relative_path}: {uses} is missing from the workflow lock entries")
                check(uses.lower() in locked_dependency_names,
                      f"{relative_path}: {uses} is missing from the dependency lock entries")

    setup_rust = yaml.safe_load(
        Path(".github/actions/setup-rust/action.yml").read_text(encoding="utf-8")
    )
    composite_refs = {
        step["uses"].lower()
        for step in (setup_rust.get("runs") or {}).get("steps") or []
        if step.get("uses") and not step["uses"].startswith(("./", "$/"))
    }
    users = [
        path.as_posix()
        for path in sorted(Path(".github/workflows").glob("*.yml"))
        if "$/.github/actions/setup-rust" in path.read_text(encoding="utf-8")
    ]
    check(bool(users), "workflows must use the local Rust composite action")
    for relative_path in users:
        locked_refs = {
            name.lower() for name in locked_workflows.get(relative_path) or []
        }
        for uses in composite_refs:
            check(uses in locked_refs,
                  f"{relative_path}: composite dependency {uses} is not locked")
            check(uses in locked_dependency_names,
                  f"composite dependency {uses} is missing from dependencies")

    verifier_job = (lock_workflow.get("jobs") or {}).get("verify") or {}
    triggers = lock_workflow.get("on") or lock_workflow.get(True) or {}
    check(lock_workflow.get("permissions") == {"contents": "read"},
          "lockfile verification must use contents: read only")
    check("pull_request" in triggers,
          "lockfile verification must run on pull requests")
    check((triggers.get("push") or {}).get("branches") == ["main"],
          "lockfile verification must run on pushes to main")
    check(bool(triggers.get("schedule")),
          "lockfile verification must periodically check upstream pins")
    check(verifier_job.get("permissions") is None,
          "the verification job must not broaden workflow permissions")
    verifier_steps = verifier_job.get("steps") or []
    check(any(
        re.fullmatch(r"actions/checkout@[a-f0-9]{40}", step.get("uses", ""))
        and (step.get("with") or {}).get("persist-credentials") is False
        for step in verifier_steps
    ), "the verifier must check out source without persisting credentials")
    check(any(
        "gh extension install github/gh-actions-lock --pin v0.4.1" in str(step.get("run", ""))
        for step in verifier_steps
    ), "the verifier must install the supported pinned gh-actions-lock extension")
    check(any(
        "gh actions-lock --verify" in str(step.get("run", ""))
        for step in verifier_steps
    ), "CI must run the lockfile verifier without writing")
    check(Path("tests/actions_lock_cli_test.sh").is_file(),
          "the stale and missing lockfile regression test must exist")
    check(any(
        "tests/actions_lock_cli_test.sh" in str(step.get("run", ""))
        for step in verifier_steps
    ), "CI must run the stale and missing lockfile regression test")

    dependabot_config = yaml.safe_load(
        Path(".github/dependabot.yml").read_text(encoding="utf-8")
    ) or {}
    action_updates = [
        update for update in dependabot_config.get("updates") or []
        if update.get("package-ecosystem") == "github-actions"
    ]
    check(len(action_updates) == 1,
          "Dependabot must keep one GitHub Actions update configuration")
    if action_updates:
        check(action_updates[0].get("schedule", {}).get("interval") == "weekly",
              "GitHub Actions Dependabot updates must remain weekly")
        check("actions" in (action_updates[0].get("groups") or {}),
              "the existing GitHub Actions Dependabot group must remain configured")


def job_names(workflow):
    return set((workflow.get("jobs") or {}).keys())


# ── Required check names are ruleset contracts ──────────────────────────────
quality_gate = ci["jobs"]["quality-gate"]
check(quality_gate["name"] == "Quality Gate (pull-request)",
      "quality-gate must keep the static ruleset name 'Quality Gate (pull-request)'")
check("${{" not in quality_gate["name"],
      "the required quality-gate name must not contain a GitHub expression")

merge_job = policy["jobs"]["merge-policy"]
check(merge_job["name"] == "PR Merge Policy Gate",
      "the merge policy job must be named exactly 'PR Merge Policy Gate'")
check("${{" not in merge_job["name"],
      "the merge policy name must not contain a GitHub expression")

# ── CI: classification happens before tool installation ─────────────────────
steps = quality_gate["steps"]
step_names = [step.get("name", "") for step in steps]
classify_index = next((i for i, n in enumerate(step_names) if n == "Classify generated maintenance PR"), None)
check(classify_index is not None, "ci.yml must classify generated maintenance PRs")
toolchain_index = next((i for i, step in enumerate(steps) if "dtolnay/rust-toolchain" in str(step.get("uses", ""))), None)
check(toolchain_index is not None, "ci.yml must still install the Rust toolchain for the full tier")
check(classify_index is not None and toolchain_index is not None and classify_index < toolchain_index,
      "maintenance classification must run before any toolchain installation")
check(classify_index == 0 or "actions/checkout" in str(steps[0].get("uses", "")),
      "the only step before classification may be the checkout")

check(any("scripts/classify-maintenance-pr.py" in str(step.get("run", "")) for step in steps),
      "ci.yml must invoke the shared maintenance classifier")
check(any("scripts/maintenance-validate.sh" in str(step.get("run", "")) for step in steps),
      "ci.yml must run the maintenance validators")

full_tier_steps = [
    step for step in steps
    if "maintenance_only != 'true'" in str(step.get("if", ""))
]
check(len(full_tier_steps) >= 5,
      "the full pull-request tier must be explicitly gated behind the classification")
check(any("quality run" in str(step.get("run", "")) for step in full_tier_steps),
      "the full tier must still run `xtask quality run` for ordinary PRs")
check(any("scripts/quality-plan-tools.py" in str(step.get("run", "")) for step in steps),
      "ci.yml must resolve the selected tool groups before installing optional tooling")

# No workflow-level path filter may suppress the required quality gate.
check("paths:" not in yaml.safe_dump(ci.get("on") or ci.get(True)),
      "ci.yml must not use workflow-level path filters on pull_request")

# ── Merge policy workflow ───────────────────────────────────────────────────
permissions = merge_job.get("permissions") or {}
check(permissions == {"contents": "read", "pull-requests": "read", "checks": "read"},
      f"merge policy permissions must be least privilege, found {sorted(permissions.items())}")
check(policy.get("permissions") == {},
      "the merge policy workflow must default to no permissions")
check("head.sha" in str(merge_job["steps"][-1].get("env", {})),
      "the merge policy gate must evaluate github.event.pull_request.head.sha")
triggers = policy.get("on") or policy.get(True) or {}
pr_types = (triggers.get("pull_request") or {}).get("types") or []
for required in ("opened", "synchronize", "reopened", "ready_for_review"):
    check(required in pr_types, f"merge policy must run on pull_request '{required}'")
check("paths:" not in policy_text,
      "the merge policy workflow must not use path filters")

# ── Dependabot auto-merge workflow ─────────────────────────────────────────
dep_triggers = dependabot.get("on") or dependabot.get(True) or {}
dep_pull_request = dep_triggers.get("pull_request") or {}
check(dep_pull_request.get("branches") == ["main"],
      "Dependabot auto-merge must only target main")
for required in ("opened", "synchronize", "reopened", "ready_for_review"):
    check(required in (dep_pull_request.get("types") or []),
          f"Dependabot auto-merge must run on pull_request '{required}'")
check(dependabot.get("permissions") == {},
      "Dependabot workflow must default to no permissions")
dep_job = dependabot["jobs"]["auto-merge"]
check(dep_job.get("if") == "github.actor == 'dependabot[bot]'",
      "only Dependabot-authored PRs may enable this auto-merge path")
check(dep_job.get("permissions") == {"contents": "write", "pull-requests": "write"},
      "Dependabot auto-merge must use only contents/pull-requests write permissions")
dep_steps = dep_job["steps"]
metadata_step = next((step for step in dep_steps if "dependabot/fetch-metadata@" in str(step.get("uses", ""))), None)
check(metadata_step is not None and re.search(r"dependabot/fetch-metadata@[0-9a-f]{40}", str(metadata_step.get("uses", ""))),
      "Dependabot metadata action must be pinned to a full commit SHA")
check(not any("gh pr review" in str(step.get("run", "")) or "--approve" in str(step.get("run", ""))
              for step in dep_steps),
      "Dependabot workflow must not auto-approve when the ruleset requires zero approvals")
dep_merge = next((step for step in dep_steps if "gh pr merge" in str(step.get("run", ""))), None)
check(dep_merge is not None and "--auto" in str(dep_merge.get("run", "")) and "--squash" in str(dep_merge.get("run", "")),
      "Dependabot must request native squash auto-merge")
dep_condition = str(dep_merge.get("if", "")) if dep_merge else ""
check("version-update:semver-minor" in dep_condition and "version-update:semver-patch" in dep_condition,
      "Dependabot auto-merge must explicitly allow minor and patch updates")
check("semver-major" not in dep_condition,
      "Dependabot auto-merge must fail closed for major or unknown update types")

# ── DORA workflow ──────────────────────────────────────────────────────────
dora_job = dora["jobs"]["dora-report"]
dora_steps = dora_job["steps"]
dora_permissions = dora_job.get("permissions") or {}
check(dora_permissions == {"contents": "write", "pull-requests": "write", "checks": "read"},
      f"DORA job permissions must be minimal for branch+PR+auto-merge, found {sorted(dora_permissions.items())}")

checkout = dora_steps[0]
check(checkout.get("with", {}).get("ref") == "main",
      "DORA must check out main explicitly before generating reports")
check(checkout.get("with", {}).get("fetch-depth") == 0,
      "DORA must keep full history for report computation/branch comparison")

check("[skip ci]" not in dora_text and "[ci skip]" not in dora_text,
      "the DORA workflow must not rely on commit-message CI skipping")

create_step = next((step for step in dora_steps if "peter-evans/create-pull-request" in str(step.get("uses", ""))), None)
check(create_step is not None, "DORA must still create/update its pull request")
if create_step is not None:
    with_block = create_step.get("with", {})
    check(with_block.get("branch") == "automation/dora-report",
          "DORA must reuse the fixed automation/dora-report branch")
    check(with_block.get("base") == "main", "DORA must target base main")
    check("id" in create_step, "the create-pull-request step needs an id for its outputs")
    check(with_block.get("delete-branch") is False,
          "DORA must not delete the fixed branch, so the existing PR is updated")

merged_text = dora_text
check("--auto" in merged_text and "--squash" in merged_text,
      "DORA must request `gh pr merge --auto --squash`")
check("--admin" not in merged_text, "DORA must never bypass rules with --admin")
check("update-branch" in merged_text,
      "DORA must refresh a stale PR branch before enabling auto-merge")
check("BEHIND" in merged_text, "DORA must only update the branch when it is behind")
check("pull-request-number != ''" in merged_text,
      "DORA merge steps must be skipped when no PR was created/updated")
check("exit 1" in merged_text, "a failed branch update must fail the workflow visibly")

# The generated PR must not be auto-approved to work around review gates.
check("gh pr review" not in merged_text and "--approve" not in merged_text,
      "DORA must not auto-approve its own PR")

if failures:
    print("FAIL: workflow maintenance contract violations", file=sys.stderr)
    for failure in failures:
        print(f"  - {failure}", file=sys.stderr)
    sys.exit(1)

print("PASS: maintenance fast path, merge policy, and DORA automation contracts hold")
PY
