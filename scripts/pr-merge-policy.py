#!/usr/bin/env python3
"""Repository-owned merge policy gate for pull requests targeting `main`.

GitHub branch rulesets cannot express conditional required checks, so the
repository replaces an unconditional Codacy context with this always-running
job. It evaluates the exact PR head SHA and decides:

* generated maintenance PR (see ``config/maintenance-pr.json``) — the DORA
  validators must have succeeded and Codacy is intentionally not required;
* every other PR — the Codacy check run must exist on *that* head SHA, come
  from the Codacy GitHub App, and conclude ``success``.

Everything fails closed. Missing files, an incomplete paginated file list, an
API error, an unknown or stale check run, a wrong app ID, a pending check past
the deadline, or an untrusted automation identity all produce a non-zero exit
with an explanatory summary.

Usage::

    python3 scripts/pr-merge-policy.py --pr 494 --head-sha <sha>
    python3 scripts/pr-merge-policy.py --pr 494 --head-sha <sha> --files-json files.json

The ``--files-json`` and ``--check-runs-json`` overrides exist so tests can
exercise every decision offline; in CI the script paginates the real API with
the ambient ``GH_TOKEN``.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
from typing import Any, Iterable

POLICY_PATH = Path("config/maintenance-pr.json")
CLASSIFIER_PATH = Path("scripts/classify-maintenance-pr.py")

TERMINAL_FAILURES = {"failure", "cancelled", "timed_out", "action_required", "stale", "startup_failure"}


def load_classifier():
    """Load the classifier module so both tools share one allowlist implementation."""
    spec = spec_from_file_location("classify_maintenance_pr", CLASSIFIER_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load classifier from {CLASSIFIER_PATH}")
    module = module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def gh_json(args: list[str]) -> Any:
    """Run `gh api` with pagination slurped into one JSON document."""
    command = ["gh", "api", "--paginate", "--slurp", *args]
    result = subprocess.run(command, capture_output=True, text=True, check=False)
    if result.returncode != 0:
        raise RuntimeError(
            f"`{' '.join(command)}` failed with exit {result.returncode}: "
            f"{result.stderr.strip() or result.stdout.strip()}"
        )
    return json.loads(result.stdout)


def fetch_pr_files(repo: str, pr: str) -> list[str] | None:
    """Fetch the complete paginated changed-file list, or `None` when incomplete."""
    try:
        pages = gh_json([f"repos/{repo}/pulls/{pr}/files?per_page=100"])
    except RuntimeError as error:
        print(f"::error::could not read the complete changed-file list: {error}")
        return None
    if not isinstance(pages, list):
        return None
    files: list[str] = []
    for page in pages:
        if not isinstance(page, list):
            return None
        for entry in page:
            if not isinstance(entry, dict) or not isinstance(entry.get("filename"), str):
                return None
            files.append(entry["filename"])
    return files or None


def normalize_check_runs(payload: Any) -> list[dict[str, Any]] | None:
    """Flatten a check-runs payload (single object, page list, or slurped pages).

    Returns `None` when the payload is unusable or provably truncated. The
    truncation guard matters: a partial list could hide a failed required check,
    so an API `total_count` larger than the runs collected refuses to decide.
    """
    if payload is None:
        return None
    if isinstance(payload, list):
        if not payload:
            return None
        # A bare list of check runs (tests and `--jq` callers) is distinguishable
        # from a list of paginated response documents by the entry shape.
        if all(isinstance(entry, dict) and "check_runs" not in entry for entry in payload):
            if not all("name" in entry for entry in payload):
                return None
            return payload
        documents: list[Any] = payload
    else:
        documents = [payload]

    runs: list[dict[str, Any]] = []
    total: int | None = None
    for document in documents:
        if not isinstance(document, dict) or not isinstance(document.get("check_runs"), list):
            return None
        if isinstance(document.get("total_count"), int):
            total = document["total_count"]
        runs.extend(entry for entry in document["check_runs"] if isinstance(entry, dict))
    if total is not None and total > len(runs):
        print(f"::error::check-run list is incomplete ({len(runs)} of {total}); refusing to decide")
        return None
    return runs


def fetch_check_runs(repo: str, sha: str) -> list[dict[str, Any]] | None:
    """Fetch every check run for `sha`, or `None` when the response is unexpected."""
    try:
        payload = gh_json([f"repos/{repo}/commits/{sha}/check-runs?filter=all&per_page=100"])
    except RuntimeError as error:
        print(f"::error::could not read check runs for {sha}: {error}")
        return None
    return normalize_check_runs(payload)


def select_best_run(runs: Iterable[dict[str, Any]], name: str, app_id: int) -> dict[str, Any] | None:
    """Pick the most advanced run for `name` from `app_id`, preferring a success."""
    matching = [
        run
        for run in runs
        if run.get("name") == name and (run.get("app") or {}).get("id") == app_id
    ]
    if not matching:
        return None
    for preferred in ("success", "failure", *sorted(TERMINAL_FAILURES)):
        for run in matching:
            if run.get("conclusion") == preferred:
                return run
    return matching[0]


def wait_for_check(
    fetch: Any,
    name: str,
    app_id: int,
    *,
    timeout_seconds: int,
    poll_seconds: int,
    sleep: Any = time.sleep,
    clock: Any = time.monotonic,
) -> tuple[str, dict[str, Any] | None, str]:
    """Poll for a terminal check run. Returns `(result, run, detail)`."""
    deadline = clock() + timeout_seconds
    last_status = "missing"
    while True:
        runs = fetch()
        if runs is None:
            return "api_error", None, "the check-run API did not return a usable response"
        run = select_best_run(runs, name, app_id)
        if run is not None:
            status = run.get("status")
            conclusion = run.get("conclusion")
            if conclusion in TERMINAL_FAILURES:
                return "failure", run, f"conclusion {conclusion!r}"
            if status == "completed" and conclusion == "success":
                return "success", run, "conclusion 'success'"
            last_status = f"status={status!r} conclusion={conclusion!r}"
        if clock() >= deadline:
            return (
                "timeout",
                run,
                f"no successful '{name}' run from app {app_id} within {timeout_seconds}s "
                f"(last observed: {last_status})",
            )
        sleep(poll_seconds)


def decide(
    *,
    classifier: Any,
    policy: dict[str, Any],
    files: list[str] | None,
    actor: str | None,
    head_ref: str | None,
    base_ref: str | None,
    event: str | None,
    check_runs: list[dict[str, Any]] | None,
) -> dict[str, Any]:
    """Pure policy decision shared by the CLI and the tests."""
    codacy = policy["codacy"]
    decision: dict[str, Any] = {
        "policy": "codacy-required",
        "passed": False,
        "summary": "",
        "classification": None,
        "codacy_run": None,
    }

    classification = classifier.classify(
        files,
        actor=actor,
        head_ref=head_ref,
        base_ref=base_ref,
        event=event,
        policy=policy,
    )
    decision["classification"] = classification

    if classification["maintenance_only"]:
        decision["policy"] = "maintenance-allowlisted"
        if check_runs is None:
            decision["summary"] = (
                "maintenance classification succeeded but the quality gate result "
                "could not be verified; failing closed"
            )
            return decision
        gate = select_best_run(
            check_runs, policy["quality_gate_check_name"], policy["github_actions_app_id"]
        )
        if gate is None:
            decision["summary"] = (
                f"maintenance PR but '{policy['quality_gate_check_name']}' has not reported; "
                "failing closed. Codacy is intentionally not required for allowlisted files."
            )
            return decision
        decision["codacy_run"] = gate
        if gate.get("conclusion") != "success" or gate.get("status") != "completed":
            decision["summary"] = (
                f"'{policy['quality_gate_check_name']}' concluded "
                f"{gate.get('conclusion')!r} ({gate.get('status')!r}); the maintenance "
                "fast path requires a successful quality gate"
            )
            return decision
        decision["passed"] = True
        decision["summary"] = (
            "generated maintenance PR: every changed file matches the reviewed allowlist, "
            "the required quality gate passed, and Codacy is intentionally not required"
        )
        return decision

    decision["policy"] = "codacy-required"
    if check_runs is None:
        decision["summary"] = "Codacy result could not be read; failing closed"
        return decision
    run = select_best_run(check_runs, codacy["check_name"], codacy["app_id"])
    decision["codacy_run"] = run
    if run is None:
        decision["summary"] = (
            f"no '{codacy['check_name']}' check run from Codacy app {codacy['app_id']} "
            f"on this head SHA; failing closed"
        )
        return decision
    if run.get("status") != "completed" or run.get("conclusion") != "success":
        decision["summary"] = (
            f"'{codacy['check_name']}' is {run.get('status')!r} / "
            f"{run.get('conclusion')!r}; only a completed success satisfies the policy"
        )
        return decision

    decision["passed"] = True
    decision["summary"] = (
        f"ordinary PR: '{codacy['check_name']}' from app {codacy['app_id']} "
        "succeeded on the current head SHA"
    )
    return decision


def write_summary(decision: dict[str, Any], *, head_sha: str, summary_path: Path | None) -> None:
    """Publish the decision to the GitHub step summary and stdout."""
    classification = decision.get("classification") or {}
    codacy = decision.get("codacy_run") or {}
    lines = [
        "## PR Merge Policy Gate",
        "",
        f"- **Decision:** {'PASS' if decision['passed'] else 'FAIL'}",
        f"- **Policy applied:** `{decision['policy']}`",
        f"- **Head SHA:** `{head_sha}`",
        f"- **Maintenance only:** `{classification.get('maintenance_only')}`",
        f"- **Classification reason:** {classification.get('reason') or 'n/a'}",
        f"- **Changed-file digest:** `{classification.get('changed_files_digest') or 'n/a'}`",
        f"- **Enforced check:** `{codacy.get('name', 'n/a')}` "
        f"(app `{(codacy.get('app') or {}).get('id', 'n/a')}`, "
        f"conclusion `{codacy.get('conclusion', 'n/a')}`)",
        f"- **Summary:** {decision['summary']}",
    ]
    text = "\n".join(lines) + "\n"
    print(text)
    if summary_path is not None:
        with open(summary_path, "a", encoding="utf-8") as handle:
            handle.write(text)


def main(argv: list[str] | None = None) -> int:
    """Entry point for the `PR Merge Policy Gate` workflow job."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--repo", default=os.environ.get("GITHUB_REPOSITORY"))
    parser.add_argument("--pr", required=True)
    parser.add_argument("--head-sha", required=True)
    parser.add_argument("--actor")
    parser.add_argument("--head-ref")
    parser.add_argument("--base-ref")
    parser.add_argument("--event", default="pull_request")
    parser.add_argument("--files-json", type=Path)
    parser.add_argument("--check-runs-json", type=Path)
    parser.add_argument("--policy", type=Path, default=POLICY_PATH)
    parser.add_argument("--summary-file", type=Path)
    args = parser.parse_args(argv)

    classifier = load_classifier()
    try:
        policy = classifier.load_policy(args.policy)
    except classifier.PolicyError as error:
        print(f"::error::merge policy configuration rejected: {error}")
        return 1

    if args.files_json is not None:
        files = classifier.load_file_list(None, args.files_json)
    elif args.repo:
        files = fetch_pr_files(args.repo, args.pr)
    else:
        files = None

    # Classify before touching check runs so the gate knows which context it owes.
    provisional = classifier.classify(
        files,
        actor=args.actor,
        head_ref=args.head_ref,
        base_ref=args.base_ref,
        event=args.event,
        policy=policy,
    )
    if provisional["maintenance_only"]:
        required_name = policy["quality_gate_check_name"]
        required_app = policy["github_actions_app_id"]
    else:
        required_name = policy["codacy"]["check_name"]
        required_app = policy["codacy"]["app_id"]

    check_runs: list[dict[str, Any]] | None
    if args.check_runs_json is not None:
        check_runs = normalize_check_runs(json.loads(args.check_runs_json.read_text(encoding="utf-8")))
    elif args.repo:
        outcome, _run, detail = wait_for_check(
            lambda: fetch_check_runs(args.repo, args.head_sha),
            required_name,
            required_app,
            timeout_seconds=policy.get("policy_timeout_seconds", 900),
            poll_seconds=policy.get("policy_poll_seconds", 30),
        )
        if outcome == "timeout":
            print(f"::error::{detail}")
        check_runs = fetch_check_runs(args.repo, args.head_sha)
    else:
        check_runs = None

    decision = decide(
        classifier=classifier,
        policy=policy,
        files=files,
        actor=args.actor,
        head_ref=args.head_ref,
        base_ref=args.base_ref,
        event=args.event,
        check_runs=check_runs,
    )
    write_summary(decision, head_sha=args.head_sha, summary_path=args.summary_file)

    if not decision["passed"]:
        print(f"::error::PR merge policy gate failed: {decision['summary']}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
