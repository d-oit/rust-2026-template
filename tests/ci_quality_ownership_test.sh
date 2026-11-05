#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

python3 - <<'PY'
import json
import re
from pathlib import Path


def jobs_from(path):
    lines = Path(path).read_text(encoding="utf-8").splitlines()
    jobs_start = next(
        index for index, line in enumerate(lines) if line.strip() == "jobs:"
    )
    jobs = {}
    current_name = None
    current_lines = []
    for line in lines[jobs_start + 1 :]:
        if line and not line.startswith(" "):
            break
        match = re.fullmatch(r"  ([a-zA-Z0-9_-]+):\s*", line)
        if match:
            if current_name is not None:
                jobs[current_name] = "\n".join(current_lines)
            current_name = match.group(1)
            current_lines = []
        elif current_name is not None:
            current_lines.append(line)
    if current_name is not None:
        jobs[current_name] = "\n".join(current_lines)
    return jobs


ci_path = ".github/workflows/ci.yml"
ci_text = Path(ci_path).read_text(encoding="utf-8")
ci_jobs = jobs_from(ci_path)
for duplicate_job in ("fmt", "clippy", "test", "msrv", "roast"):
    assert duplicate_job not in ci_jobs, (
        f"{duplicate_job} must not be a separate blocking quality job"
    )
assert "quality run" in ci_jobs["quality-gate"]
assert "quality-gate" in ci_jobs["ci-success"]
assert "XTASK_TEST_COVERAGE: \"1\"" in ci_jobs["quality-gate"]
assert "codecov/codecov-action@" in ci_jobs["quality-gate"]
assert "name: Quality Gate (pull-request)" in ci_jobs["quality-gate"]
assert "name: CI Success" in ci_jobs["ci-success"]
direct_checks = re.compile(
    r"(?m)^\s*(?:run:\s*)?cargo\s+(?:fmt|clippy|nextest|llvm-cov|test|audit|deny)\b"
)
assert not direct_checks.search(ci_text), (
    "blocking Cargo checks must be invoked only by xtask, not duplicated in jobs"
)
assert not re.search(r"(?m)^\s*contents:\s*write\s*$", ci_text), (
    "default CI must not grant repository write permission"
)
assert not re.search(r"(?m)^\s*(?:run:\s*)?git\s+(?:config|commit|push)\b", ci_text), (
    "default CI must not commit or push generated status"
)
assert ".agents/ci/ci-status.json" in ci_jobs["ci-success"]
assert '.agents/ci/ci-summary.md >> "$GITHUB_STEP_SUMMARY"' in ci_jobs["ci-success"]
assert "needs.roast" not in ci_text and "Roast Scorer" not in ci_text

security_jobs = jobs_from(".github/workflows/security-scan.yml")
for job in ("cargo-audit", "cargo-deny"):
    condition = re.search(
        r"(?m)^\s*if:\s*(.+)$", security_jobs[job]
    )
    assert condition and "github.event_name == 'schedule'" in condition.group(1)
    assert "github.event_name == 'workflow_dispatch'" in condition.group(1)

quality = json.loads(Path("config/xtask.json").read_text(encoding="utf-8"))
for obsolete in ("CiStatusArtifact", "RoastScorer"):
    assert all(
        obsolete not in tier["checks"] for tier in quality["tiers"].values()
    ), f"{obsolete} must not be configured as a quality gate"
for tier_name in ("pull-request", "protected-branch", "release"):
    checks = quality["tiers"][tier_name]["checks"]
    for check in ("Audit", "Deny"):
        assert check in checks, f"{tier_name} must assign {check} to xtask"
assert "Msrv" in quality["tiers"]["pull-request"]["checks"]
assert "Msrv" in quality["tiers"]["protected-branch"]["checks"]
assert set(quality["tiers"]["pull-request"]["required_checks"]) == {
    "Audit",
    "Deny",
    "Msrv",
    "ShellCheck",
    "MarkdownLint",
    "YamlLint",
    "WorkflowValidation",
    "ActionLint",
    "SecretScan",
}

fuzz_path = ".github/workflows/fuzz.yml"
fuzz_text = Path(fuzz_path).read_text(encoding="utf-8")
fuzz_jobs = jobs_from(fuzz_path)
assert "bash scripts/check-fuzz-deps.sh" in fuzz_jobs["fuzz-dependencies"]
for path in ("fuzz/**", "Cargo.toml", "Cargo.lock", "deny.toml"):
    assert f"'{path}'" in fuzz_text, f"fuzz dependency check must watch {path}"
assert "schedule:" in fuzz_text

print("CI quality ownership and workflow security contracts passed.")
PY
