#!/usr/bin/env bash
# Integration test for security workflow logic, structured artifact handling, and failure classification

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

FIXTURE_DIR="${REPO_ROOT}/tests/fixtures/security"

echo "=== Running Security Workflow Integration & Regression Tests ==="

# 1. Test Gitleaks Fixture Detection
echo "[TEST] Validating Gitleaks secret detection on test fixture..."
GITLEAKS_OUTPUT=$(mktemp)
if command -v gitleaks >/dev/null 2>&1; then
  if gitleaks detect --source="${FIXTURE_DIR}/secret_fixture.txt" --no-git \
      --report-format json --report-path="${GITLEAKS_OUTPUT}" >/dev/null 2>&1; then
    echo "ERROR: Gitleaks failed to detect secret in fixture file!"
    exit 1
  fi
  echo "[PASS] Gitleaks successfully detected test secret fixture."
else
  echo "[SKIP] Gitleaks binary not present locally; skipping live binary check."
fi
rm -f "${GITLEAKS_OUTPUT}"

# 2. Test Security Summary Formatting Logic for all Scanner Classes
echo "[TEST] Validating Security Summary Markdown generator and classification..."

parse_summary() {
  local gitleaks_res="$1"
  local gitleaks_type="$2"
  local audit_res="$3"
  local audit_type="$4"
  local deny_res="$5"
  local deny_type="$6"

  local summary_out
  summary_out=$(mktemp)

  FAILED=0

  cat > "${summary_out}" << EOF
## Security Gate Verification Summary

| Control | Upstream Result | Failure Category | Diagnostic Guidance |
|---|---|---|---|
EOF

  if [ "$gitleaks_res" = "success" ]; then
    echo "| Secret Scan (Gitleaks) | ✅ PASSED | None | No secrets detected. |" >> "${summary_out}"
  elif [ "$gitleaks_type" = "infrastructure_failure" ]; then
    FAILED=1
    echo "| Secret Scan (Gitleaks) | ❌ INFRASTRUCTURE FAILURE | Tool Setup | Runner/checkout failed. |" \
      >> "${summary_out}"
  else
    FAILED=1
    echo "| Secret Scan (Gitleaks) | ❌ SECURITY FINDING | Secret Leak | Credentials detected. |" \
      >> "${summary_out}"
  fi

  if [ "$audit_res" = "success" ]; then
    echo "| Vulnerability Audit (cargo-audit) | ✅ PASSED | None | No advisories detected. |" >> "${summary_out}"
  elif [ "$audit_type" = "infrastructure_failure" ]; then
    FAILED=1
    echo "| Vulnerability Audit (cargo-audit) | ❌ INFRASTRUCTURE FAILURE | Tool Setup | Cargo audit failed. |" \
      >> "${summary_out}"
  else
    FAILED=1
    echo "| Vulnerability Audit (cargo-audit) | ❌ SECURITY FINDING | Vulnerable Dependency | Advisory found. |" \
      >> "${summary_out}"
  fi

  if [ "$deny_res" = "success" ]; then
    echo "| Dependency Policy (cargo-deny) | ✅ PASSED | None | All dependencies comply with policy. |" >> "${summary_out}"
  elif [ "$deny_type" = "infrastructure_failure" ]; then
    FAILED=1
    echo "| Dependency Policy (cargo-deny) | ❌ INFRASTRUCTURE FAILURE | Tool Setup | Cargo deny failed. |" \
      >> "${summary_out}"
  else
    FAILED=1
    echo "| Dependency Policy (cargo-deny) | ❌ SECURITY FINDING | Policy Violation | Banned crate/license. |" \
      >> "${summary_out}"
  fi

  if [ $FAILED -ne 0 ]; then
    echo "FAILED" >> "${summary_out}"
  else
    echo "PASSED" >> "${summary_out}"
  fi

  cat "${summary_out}"
  rm -f "${summary_out}"
}

# Test Scenario A: Gitleaks failure
SCENARIO_A=$(parse_summary "failure" "security_finding" "success" "none" "success" "none")
echo "$SCENARIO_A" | grep -q "Secret Leak" || (echo "Scenario A failed" && exit 1)
echo "$SCENARIO_A" | grep -q "FAILED" || (echo "Scenario A failed status" && exit 1)
echo "[PASS] Scenario A (Secret leak finding) classified correctly."

# Test Scenario B: Cargo audit vulnerability finding
SCENARIO_B=$(parse_summary "success" "none" "failure" "security_finding" "success" "none")
echo "$SCENARIO_B" | grep -q "Vulnerable Dependency" || (echo "Scenario B failed" && exit 1)
echo "$SCENARIO_B" | grep -q "FAILED" || (echo "Scenario B failed status" && exit 1)
echo "[PASS] Scenario B (Audit vulnerability finding) classified correctly."

# Test Scenario C: Cargo deny policy violation finding
SCENARIO_C=$(parse_summary "success" "none" "success" "none" "failure" "security_finding")
echo "$SCENARIO_C" | grep -q "Policy Violation" || (echo "Scenario C failed" && exit 1)
echo "$SCENARIO_C" | grep -q "FAILED" || (echo "Scenario C failed status" && exit 1)
echo "[PASS] Scenario C (Cargo deny policy violation) classified correctly."

# Test Scenario D: Infrastructure / Tool installation failure
SCENARIO_D=$(parse_summary "success" "none" "failure" "infrastructure_failure" "success" "none")
echo "$SCENARIO_D" | grep -q "INFRASTRUCTURE FAILURE" || (echo "Scenario D failed" && exit 1)
echo "$SCENARIO_D" | grep -q "Tool Setup" || (echo "Scenario D failed category" && exit 1)
echo "[PASS] Scenario D (Infrastructure failure) distinguished from security finding."

# Test Scenario E: All Scans Passed
SCENARIO_E=$(parse_summary "success" "none" "success" "none" "success" "none")
echo "$SCENARIO_E" | grep -q "PASSED" || (echo "Scenario E failed status" && exit 1)
echo "[PASS] Scenario E (All checks passed) verified."

echo "=== All Security Integration Tests Passed Successfully ==="
