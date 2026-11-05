#!/usr/bin/env bash
# tests/ci_required_evaluate_test.sh
# Tests for scripts/ci-required-evaluate.sh using positive and negative fixtures.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

EVAL_SCRIPT="scripts/ci-required-evaluate.sh"
FIXTURES_DIR="tests/fixtures/ci_required"

if [[ ! -x "$EVAL_SCRIPT" ]]; then
  echo "ERROR: Evaluator script $EVAL_SCRIPT is not executable or missing."
  exit 1
fi

echo "=== Testing CI Required Job Evaluator ==="

# Test 1: Docs-only PR runs Markdown checks; unrelated jobs may skip -> PASS (0)
echo "[1/8] Testing docs-only PR with legitimate skips..."
if ! "$EVAL_SCRIPT" "${FIXTURES_DIR}/docs_only_pr_valid.json" >/dev/null 2>&1; then
  echo "FAIL: Docs-only PR fixture should have passed."
  exit 1
fi
echo "✓ Docs-only PR runs its quality gate and legitimately skips unrelated jobs: PASSED"

# Test 2: Rust source change with the canonical quality gate skipped -> FAIL
echo "[2/8] Testing Rust source change with the quality gate skipped..."
set +e
output=$("$EVAL_SCRIPT" "${FIXTURES_DIR}/rust_code_quality_gate_skipped.json" 2>&1)
exit_code=$?
set -e
if [[ $exit_code -eq 0 ]] || ! echo "$output" | grep -q "Job 'quality-gate' was skipped unexpectedly"; then
  echo "FAIL: Expected quality gate skipped error, got exit code $exit_code. Output: $output"
  exit 1
fi
echo "✓ Rust code change with quality gate skipped correctly rejected: PASSED"

# Test 3: Rust source change with the quality gate failing -> FAIL
echo "[3/8] Testing Rust source change with the quality gate failing..."
set +e
output=$("$EVAL_SCRIPT" "${FIXTURES_DIR}/rust_code_quality_gate_failed.json" 2>&1)
exit_code=$?
set -e
if [[ $exit_code -eq 0 ]] || ! echo "$output" | grep -q "Job 'quality-gate' ended with status 'failure'"; then
  echo "FAIL: Expected quality gate failure, got exit code $exit_code. Output: $output"
  exit 1
fi
echo "✓ Rust code change with quality gate failure correctly rejected: PASSED"

# Test 4: Benchmark change with bench skipped -> FAIL (non-zero)
echo "[4/8] Testing benchmark change with bench skipped..."
set +e
output=$("$EVAL_SCRIPT" "${FIXTURES_DIR}/bench_change_bench_skipped.json" 2>&1)
exit_code=$?
set -e
if [[ $exit_code -eq 0 ]] || ! echo "$output" | grep -q "Job 'bench' was skipped unexpectedly"; then
  echo "FAIL: Expected bench skipped error, got exit code $exit_code. Output: $output"
  exit 1
fi
echo "✓ Benchmark change with bench skipped correctly rejected: PASSED"

# Test 5: Agent change with validate-agents skipped -> FAIL (non-zero)
echo "[5/8] Testing agent change with validate-agents skipped..."
set +e
output=$("$EVAL_SCRIPT" "${FIXTURES_DIR}/agent_change_validate_agents_skipped.json" 2>&1)
exit_code=$?
set -e
if [[ $exit_code -eq 0 ]] || ! echo "$output" | grep -q "Job 'validate-agents' was skipped unexpectedly"; then
  echo "FAIL: Expected validate-agents skipped error, got exit code $exit_code. Output: $output"
  exit 1
fi
echo "✓ Agent change with validate-agents skipped correctly rejected: PASSED"

# Test 6: Missing and cancelled results -> FAIL (non-zero)
echo "[6/8] Testing missing and cancelled job results..."
set +e
output=$("$EVAL_SCRIPT" "${FIXTURES_DIR}/missing_and_cancelled_jobs.json" 2>&1)
exit_code=$?
set -e
if [[ $exit_code -eq 0 ]] || ! echo "$output" | grep -q "Job 'quality-gate' result is missing" || ! echo "$output" | grep -q "Job 'lint' ended with status 'cancelled'"; then
  echo "FAIL: Expected missing and cancelled errors, got exit code $exit_code. Output: $output"
  exit 1
fi
echo "✓ Missing and cancelled results correctly rejected: PASSED"

# Test 7: Changes detection job itself failed -> FAIL (non-zero)
echo "[7/8] Testing changes job failure..."
set +e
output=$("$EVAL_SCRIPT" "${FIXTURES_DIR}/changes_job_failed.json" 2>&1)
exit_code=$?
set -e
if [[ $exit_code -eq 0 ]] || ! echo "$output" | grep -q "Changes detection job did not succeed"; then
  echo "FAIL: Expected changes job failure error, got exit code $exit_code. Output: $output"
  exit 1
fi
echo "✓ Changes job failure correctly rejected: PASSED"

# Test 8: Non-PR event with skipped jobs -> FAIL (non-zero)
echo "[8/8] Testing push event with skipped jobs..."
set +e
output=$("$EVAL_SCRIPT" <<'EOF' 2>&1
{
  "event_name": "push",
  "changes_result": "success",
  "changes": {
    "code": "false",
    "heavy": "false",
    "agents": "false",
    "workflows": "false"
  },
  "jobs": {
    "validate-agents": "skipped",
    "lint": "success",
    "bench": "success",
    "version-check": "success",
    "quality-gate": "success"
  }
}
EOF
)
exit_code=$?
set -e
if [[ $exit_code -eq 0 ]] || ! echo "$output" | grep -q "Job 'validate-agents' was skipped on non-PR event 'push'"; then
  echo "FAIL: Expected non-PR skip rejection, got exit code $exit_code. Output: $output"
  exit 1
fi
echo "✓ Push event with skipped jobs correctly rejected: PASSED"

echo "=== All CI Required Job Evaluator tests PASSED successfully ==="
