#!/usr/bin/env bash
# tests/pr_merge_policy_test.sh
# Offline contract tests for the repository-owned PR Merge Policy Gate.
#
# The gate replaces an unconditional Codacy requirement with a conditional one.
# These cases pin every branch of that decision: allowlisted maintenance PRs,
# ordinary PRs, mixed changes, untrusted identities, stale or wrong-app check
# runs, pending/failed/timeout outcomes, and API failures. No network access and
# no credentials are required; fixtures stand in for the REST payloads.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

POLICY_SCRIPT="scripts/pr-merge-policy.py"
WORK="$(mktemp -d)"
FIXTURES="$WORK/fixtures"
trap 'rm -rf "$WORK"' EXIT

PASS=0
FAIL=0

ok() {
  PASS=$((PASS + 1))
  echo "✓ $1"
}

bad() {
  FAIL=$((FAIL + 1))
  echo "✗ $1" >&2
}

# run_gate <files-json> <check-runs-json> [extra args...]
run_gate() {
  local files="$1" runs="$2"
  shift 2
  python3 "$POLICY_SCRIPT" \
    --pr 494 \
    --head-sha 1111111111111111111111111111111111111111 \
    --files-json "$files" \
    --check-runs-json "$runs" \
    --summary-file "$WORK/summary.md" \
    "$@"
}

assert_gate() {
  # assert_gate <expected-exit> <description> <files> <runs> [extra args...]
  local expected="$1" description="$2" files="$3" runs="$4"
  shift 4
  local actual=0
  run_gate "$files" "$runs" "$@" >"$WORK/out.txt" 2>&1 || actual=$?
  if [[ "$actual" -eq "$expected" ]]; then
    ok "$description"
  else
    bad "$description (expected exit $expected, got $actual)"
    sed 's/^/    /' "$WORK/out.txt" >&2
  fi
}

assert_gate_says() {
  # assert_gate_says <needle> <description> <files> <runs> [extra args...]
  local needle="$1" description="$2" files="$3" runs="$4"
  shift 4
  local actual=0
  run_gate "$files" "$runs" "$@" >"$WORK/out.txt" 2>&1 || actual=$?
  if grep -qF -- "$needle" "$WORK/out.txt"; then
    ok "$description"
  else
    bad "$description (missing: $needle)"
    sed 's/^/    /' "$WORK/out.txt" >&2
  fi
}

BOT_ACTOR='github-actions[bot]'
BOT_BRANCH='automation/dora-report'

dora_files='["reports/DORA-REPORT.md","reports/dora-history.jsonl","reports/dora-manifest.json"]'
code_files='["crates/sample-app/src/main.rs","reports/DORA-REPORT.md"]'

# Fixtures are generated from the real payload shape GitHub returns.
python3 - "$FIXTURES" "$WORK" <<'PY'
import json
import sys
from pathlib import Path

fixtures = Path(sys.argv[1])
work = Path(sys.argv[2])
fixtures.mkdir(parents=True, exist_ok=True)


def run(name, app_id, conclusion, status="completed", sha="1111111111111111111111111111111111111111"):
    return {
        "id": hash((name, app_id, conclusion, status)) % 10_000,
        "name": name,
        "head_sha": sha,
        "status": status,
        "conclusion": conclusion,
        "app": {"id": app_id, "slug": "fixture"},
    }


CODACY_NAME = "Codacy Static Code Analysis"
CODACY_APP = 56611
GATE_NAME = "Quality Gate (pull-request)"
ACTIONS_APP = 15368

cases = {
    "codacy_success": [run(CODACY_NAME, CODACY_APP, "success"), run(GATE_NAME, ACTIONS_APP, "success")],
    "codacy_failure": [run(CODACY_NAME, CODACY_APP, "failure"), run(GATE_NAME, ACTIONS_APP, "success")],
    "codacy_pending": [run(CODACY_NAME, CODACY_APP, None, status="in_progress")],
    "codacy_missing": [run(GATE_NAME, ACTIONS_APP, "success")],
    "codacy_wrong_app": [run(CODACY_NAME, 99999, "success"), run(GATE_NAME, ACTIONS_APP, "success")],
    "codacy_cancelled": [run(CODACY_NAME, CODACY_APP, "cancelled")],
    "codacy_timed_out": [run(CODACY_NAME, CODACY_APP, "timed_out")],
    "gate_success": [run(GATE_NAME, ACTIONS_APP, "success")],
    "gate_failure": [run(GATE_NAME, ACTIONS_APP, "failure")],
    "gate_missing": [run(CODACY_NAME, CODACY_APP, "success")],
    "gate_wrong_app": [run(GATE_NAME, 99999, "success")],
}
for name, payload in cases.items():
    (fixtures / f"{name}.json").write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")

# A truncated check-run page: total_count exceeds the runs actually returned.
truncated = {"total_count": 42, "check_runs": [run(CODACY_NAME, CODACY_APP, "success")]}
(work / "truncated.json").write_text(json.dumps(truncated), encoding="utf-8")

# An API error payload is an object, not a list: the gate must refuse to decide.
(work / "api_error.json").write_text(json.dumps({"message": "Server Error"}), encoding="utf-8")
PY

echo "=== Maintenance allowlist policy ==="

assert_gate 0 "allowlisted DORA PR passes once the quality gate succeeded" \
  <(echo "$dora_files") "$FIXTURES/gate_success.json" \
  --actor "$BOT_ACTOR" --head-ref "$BOT_BRANCH" --base-ref main

assert_gate_says "maintenance-allowlisted" \
  "summary names the maintenance policy" \
  <(echo "$dora_files") "$FIXTURES/gate_success.json" \
  --actor "$BOT_ACTOR" --head-ref "$BOT_BRANCH" --base-ref main

assert_gate_says "Codacy is intentionally not required" \
  "summary explains the intentional Codacy exception" \
  <(echo "$dora_files") "$FIXTURES/gate_success.json" \
  --actor "$BOT_ACTOR" --head-ref "$BOT_BRANCH" --base-ref main

assert_gate 1 "allowlisted PR still requires a successful quality gate" \
  <(echo "$dora_files") "$FIXTURES/gate_failure.json" \
  --actor "$BOT_ACTOR" --head-ref "$BOT_BRANCH" --base-ref main

assert_gate 1 "allowlisted PR fails when the quality gate has not reported" \
  <(echo "$dora_files") "$FIXTURES/gate_missing.json" \
  --actor "$BOT_ACTOR" --head-ref "$BOT_BRANCH" --base-ref main

assert_gate 1 "quality gate from an unexpected app does not satisfy the policy" \
  <(echo "$dora_files") "$FIXTURES/gate_wrong_app.json" \
  --actor "$BOT_ACTOR" --head-ref "$BOT_BRANCH" --base-ref main

assert_gate 1 "allowlisted PR without check-run data fails closed" \
  <(echo "$dora_files") "$WORK/api_error.json" \
  --actor "$BOT_ACTOR" --head-ref "$BOT_BRANCH" --base-ref main

echo ""
echo "=== Ordinary PRs still require Codacy ==="

assert_gate 0 "ordinary PR passes with a successful current-head Codacy run" \
  <(echo "$code_files") "$FIXTURES/codacy_success.json" \
  --actor 'd-oit' --head-ref 'feat/something' --base-ref main

assert_gate 1 "failed Codacy run blocks the PR" \
  <(echo "$code_files") "$FIXTURES/codacy_failure.json" \
  --actor 'd-oit' --head-ref 'feat/something' --base-ref main

assert_gate 1 "pending Codacy run does not pass" \
  <(echo "$code_files") "$FIXTURES/codacy_pending.json" \
  --actor 'd-oit' --head-ref 'feat/something' --base-ref main

assert_gate 1 "missing Codacy run fails closed" \
  <(echo "$code_files") "$FIXTURES/codacy_missing.json" \
  --actor 'd-oit' --head-ref 'feat/something' --base-ref main

assert_gate 1 "Codacy run from a different app fails closed" \
  <(echo "$code_files") "$FIXTURES/codacy_wrong_app.json" \
  --actor 'd-oit' --head-ref 'feat/something' --base-ref main

assert_gate 1 "cancelled Codacy run blocks the PR" \
  <(echo "$code_files") "$FIXTURES/codacy_cancelled.json" \
  --actor 'd-oit' --head-ref 'feat/something' --base-ref main

assert_gate 1 "timed-out Codacy run blocks the PR" \
  <(echo "$code_files") "$FIXTURES/codacy_timed_out.json" \
  --actor 'd-oit' --head-ref 'feat/something' --base-ref main

assert_gate 1 "incomplete check-run pagination fails closed" \
  <(echo "$code_files") "$WORK/truncated.json" \
  --actor 'd-oit' --head-ref 'feat/something' --base-ref main

assert_gate 1 "GitHub API error payload fails closed" \
  <(echo "$code_files") "$WORK/api_error.json" \
  --actor 'd-oit' --head-ref 'feat/something' --base-ref main

echo ""
echo "=== User-controlled inputs cannot force the exception ==="

assert_gate 0 "mixed DORA + source change still enforces Codacy, which passed here" \
  <(echo "$code_files") "$FIXTURES/codacy_success.json" \
  --actor "$BOT_ACTOR" --head-ref "$BOT_BRANCH" --base-ref main

assert_gate_says "codacy-required" "mixed change is classified as an ordinary PR" \
  <(echo "$code_files") "$FIXTURES/gate_success.json" \
  --actor "$BOT_ACTOR" --head-ref "$BOT_BRANCH" --base-ref main

assert_gate 1 "mixed change with failing Codacy stays blocked" \
  <(echo "$code_files") "$FIXTURES/codacy_failure.json" \
  --actor "$BOT_ACTOR" --head-ref "$BOT_BRANCH" --base-ref main

assert_gate 1 "same file set from a human branches to the Codacy policy" \
  <(echo "$dora_files") "$FIXTURES/gate_success.json" \
  --actor 'someone-else' --head-ref 'automation/dora-report' --base-ref main

assert_gate 1 "same file set from an untrusted branch requires Codacy" \
  <(echo "$dora_files") "$FIXTURES/gate_success.json" \
  --actor "$BOT_ACTOR" --head-ref 'feature/dora-copy' --base-ref main

assert_gate 1 "same file set targeting a non-main base requires Codacy" \
  <(echo "$dora_files") "$FIXTURES/gate_success.json" \
  --actor "$BOT_ACTOR" --head-ref "$BOT_BRANCH" --base-ref develop

assert_gate 1 "unavailable file list reverts to the Codacy policy" \
  /dev/null "$FIXTURES/codacy_missing.json" \
  --actor "$BOT_ACTOR" --head-ref "$BOT_BRANCH" --base-ref main

echo ""
echo "=== Configuration drift fails closed ==="

jq '.codacy.app_id = 1' config/maintenance-pr.json >"$WORK/other-codacy.json"
assert_gate 1 "policy with a different Codacy app ID rejects the real Codacy run" \
  <(echo "$code_files") "$FIXTURES/codacy_success.json" \
  --actor 'd-oit' --head-ref 'feat/something' --base-ref main \
  --policy "$WORK/other-codacy.json"

jq 'del(.allowed_paths)' config/maintenance-pr.json >"$WORK/broken-policy.json"
assert_gate 1 "policy missing the allowlist is rejected" \
  <(echo "$dora_files") "$FIXTURES/gate_success.json" \
  --actor "$BOT_ACTOR" --head-ref "$BOT_BRANCH" --base-ref main \
  --policy "$WORK/broken-policy.json"

echo ""
echo "=== Summary contract ==="

run_gate <(echo "$dora_files") "$FIXTURES/gate_success.json" \
  --actor "$BOT_ACTOR" --head-ref "$BOT_BRANCH" --base-ref main >/dev/null 2>&1 || true
for field in "Head SHA" "Policy applied" "Maintenance only" "Changed-file digest" "Enforced check"; do
  if grep -qF -- "$field" "$WORK/summary.md" 2>/dev/null; then
    ok "step summary reports $field"
  else
    bad "step summary is missing $field"
  fi
done

if grep -qF '1111111111111111111111111111111111111111' "$WORK/summary.md" 2>/dev/null; then
  ok "step summary pins the evaluated head SHA"
else
  bad "step summary does not record the evaluated head SHA"
fi

echo ""
if [[ $FAIL -gt 0 ]]; then
  echo "FAIL: $FAIL assertion(s) failed, $PASS passed." >&2
  exit 1
fi
echo "PASS: all $PASS PR merge policy assertions passed."
