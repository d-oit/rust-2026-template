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

python3 - "$CI" "$DORA" "$POLICY" <<'PY'
import re
import sys
from pathlib import Path

import yaml

ci_path, dora_path, policy_path = (Path(arg) for arg in sys.argv[1:4])
ci_text = ci_path.read_text(encoding="utf-8")
dora_text = dora_path.read_text(encoding="utf-8")
policy_text = policy_path.read_text(encoding="utf-8")

ci = yaml.safe_load(ci_text)
dora = yaml.safe_load(dora_text)
policy = yaml.safe_load(policy_text)

failures = []


def check(condition, message):
    if not condition:
        failures.append(message)


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
