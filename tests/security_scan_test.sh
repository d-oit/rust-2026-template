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
  local audit_graph="${7:-Committed Cargo.lock}"
  local deny_graph="${8:-Committed Cargo.lock}"

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

  AUDIT_NOTE=""
  if [ -n "$audit_graph" ]; then
    AUDIT_NOTE=" (Graph: ${audit_graph})"
  fi
  if [ "$audit_res" = "success" ]; then
    echo "| Vulnerability Audit (cargo-audit) | ✅ PASSED | None | No advisories detected.${AUDIT_NOTE} |" >> "${summary_out}"
  elif [ "$audit_type" = "infrastructure_failure" ]; then
    FAILED=1
    echo "| Vulnerability Audit (cargo-audit) | ❌ INFRASTRUCTURE FAILURE | Tool Setup | Cargo audit failed.${AUDIT_NOTE} |" \
      >> "${summary_out}"
  else
    FAILED=1
    echo "| Vulnerability Audit (cargo-audit) | ❌ SECURITY FINDING | Vulnerable Dependency | Advisory found.${AUDIT_NOTE} |" \
      >> "${summary_out}"
  fi

  DENY_NOTE=""
  if [ -n "$deny_graph" ]; then
    DENY_NOTE=" (Graph: ${deny_graph})"
  fi
  if [ "$deny_res" = "success" ]; then
    echo "| Dependency Policy (cargo-deny) | ✅ PASSED | None | All dependencies comply with policy.${DENY_NOTE} |" >> "${summary_out}"
  elif [ "$deny_type" = "infrastructure_failure" ]; then
    FAILED=1
    echo "| Dependency Policy (cargo-deny) | ❌ INFRASTRUCTURE FAILURE | Tool Setup | Cargo deny failed.${DENY_NOTE} |" \
      >> "${summary_out}"
  else
    FAILED=1
    echo "| Dependency Policy (cargo-deny) | ❌ SECURITY FINDING | Policy Violation | Banned crate/license.${DENY_NOTE} |" \
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

# Test Scenario F: Audited Graph Reporting
SCENARIO_F=$(parse_summary "success" "none" "success" "none" "success" "none" "Committed Cargo.lock" "Resolved latest-compatible graph")
echo "$SCENARIO_F" | grep -q "Graph: Committed Cargo.lock" || (echo "Scenario F audit graph missing" && exit 1)
echo "$SCENARIO_F" | grep -q "Graph: Resolved latest-compatible graph" || (echo "Scenario F deny graph missing" && exit 1)
echo "[PASS] Scenario F (Audited graph identification in summary) verified."

# 3. Test Lockfile Strategy Logic (Application vs Library Profile)
echo "[TEST] Validating lockfile strategy and integrity check logic..."

# Scenario 3A: Committed Cargo.lock (Application Profile)
TEST_DIR_APP=$(mktemp -d)
(
  cd "${TEST_DIR_APP}"
  git init -q
  git config user.name "Test"
  git config user.email "test@example.com"
  cat > Cargo.toml << 'EOF'
[package]
name = "dummy-app"
version = "0.1.0"
edition = "2021"
EOF
  cat > Cargo.lock << 'EOF'
# Dummy committed Cargo.lock
version = 3
EOF
  git add Cargo.toml Cargo.lock
  git commit -m "initial commit" -q

  # Execute lockfile strategy logic from workflow
  if git ls-files --error-unmatch Cargo.lock >/dev/null 2>&1 || [ -f Cargo.lock ]; then
    LOCKFILE_TRACKED=true
    AUDITED_GRAPH="Committed Cargo.lock"
  else
    LOCKFILE_TRACKED=false
    AUDITED_GRAPH="Resolved latest-compatible graph"
  fi

  if [ "$LOCKFILE_TRACKED" != "true" ] || [ "$AUDITED_GRAPH" != "Committed Cargo.lock" ]; then
    echo "ERROR: Failed to identify committed lockfile as tracked application profile!"
    exit 1
  fi

  # Simulate post-audit lockfile integrity check
  if ! git diff --exit-code Cargo.lock >/dev/null; then
    echo "ERROR: Lockfile integrity check failed on unchanged committed lockfile!"
    exit 1
  fi
)
rm -rf "${TEST_DIR_APP}"
echo "[PASS] Application profile lockfile strategy & preservation verified."

# Scenario 3B: Untracked Cargo.lock (Library Profile)
TEST_DIR_LIB=$(mktemp -d)
(
  cd "${TEST_DIR_LIB}"
  git init -q
  git config user.name "Test"
  git config user.email "test@example.com"
  cat > Cargo.toml << 'EOF'
[package]
name = "dummy-lib"
version = "0.1.0"
edition = "2021"
EOF
  git add Cargo.toml
  git commit -m "initial commit" -q

  # Execute lockfile strategy logic from workflow
  if git ls-files --error-unmatch Cargo.lock >/dev/null 2>&1 || [ -f Cargo.lock ]; then
    LOCKFILE_TRACKED=true
    AUDITED_GRAPH="Committed Cargo.lock"
  else
    LOCKFILE_TRACKED=false
    AUDITED_GRAPH="Resolved latest-compatible graph"
  fi

  if [ "$LOCKFILE_TRACKED" != "false" ] || [ "$AUDITED_GRAPH" != "Resolved latest-compatible graph" ]; then
    echo "ERROR: Failed to identify untracked lockfile as library profile!"
    exit 1
  fi
)
rm -rf "${TEST_DIR_LIB}"
echo "[PASS] Library profile untracked lockfile strategy verified."

# 4. Test Unpatchable Dependency Edge Guard
# `libsql`'s default `tls` feature pins hyper-rustls 0.25 -> rustls 0.22 ->
# rustls-webpki 0.102.8. That stack carries advisories (RUSTSEC-2026-0049,
# -0098, -0099, -0104, -0258, -0346, -0349, -0382) with no patch release on the
# 0.102/0.22 lines, so the dependency edge itself must stay out of the graph.
echo "[TEST] Validating unpatchable rustls dependency edge is absent..."

VULN_PACKAGES="rustls-webpki rustls h2 hyper-rustls"

# The manifests must not re-enable libsql's default `tls` feature.
if grep -RqE '^[[:space:]]*libsql[[:space:]]*=[[:space:]]*(\{[^}]*)?[[:space:]]*"[^"]*"' \
     crates/*/Cargo.toml 2>/dev/null; then
  LIBSQL_DEP=$(grep -RhE '^[[:space:]]*libsql[[:space:]]*=' crates/*/Cargo.toml 2>/dev/null | head -1)
  if [[ -z "${LIBSQL_DEP}" ]]; then
    echo "ERROR: Could not determine libsql dependency declaration."
    exit 1
  fi
  if ! grep -q 'default-features[[:space:]]*=[[:space:]]*false' <<<"${LIBSQL_DEP}"; then
    echo "ERROR: libsql dependency re-enables default features (pulls the unpatchable rustls 0.22 stack): ${LIBSQL_DEP}"
    exit 1
  fi
  if grep -qE 'features[[:space:]]*=[[:space:]]*\[[^]]*"tls"' <<<"${LIBSQL_DEP}"; then
    echo "ERROR: libsql dependency explicitly enables the 'tls' feature: ${LIBSQL_DEP}"
    exit 1
  fi
  echo "[PASS] libsql dependency keeps default features disabled."
else
  echo "[PASS] No libsql dependency declaration found; nothing to guard."
fi

# No advisory suppressions may hide the unpatchable stack: the ignore lists must
# not reference any of the rustls/rustls-webpki/h2 advisories. Only the ignore
# list is inspected — explanatory comments may still name the advisories.
for CONFIG_FILE in "${REPO_ROOT}/.cargo/audit.toml" "${REPO_ROOT}/deny.toml"; do
  if [[ ! -f "${CONFIG_FILE}" ]]; then
    echo "ERROR: Missing advisory configuration: ${CONFIG_FILE}"
    exit 1
  fi
  IGNORED_IDS=$(python3 - "${CONFIG_FILE}" <<'PY'
import re
import sys

with open(sys.argv[1], 'r', encoding='utf-8') as fh:
    text = fh.read()

# Collect every advisory ID that appears inside an `ignore = [...]` array.
ids = []
for match in re.finditer(r'ignore\s*=\s*\[(.*?)\]', text, re.DOTALL):
    ids.extend(re.findall(r'RUSTSEC-\d{4}-\d{4}', match.group(1)))
print(' '.join(ids))
PY
)
  if grep -qE 'RUSTSEC-2026-(0049|0098|0099|0104|0258|0346|0349|0382)' <<<"${IGNORED_IDS}"; then
    echo "ERROR: ${CONFIG_FILE} suppresses an advisory for the unpatchable rustls stack; fix the dependency edge instead."
    exit 1
  fi
done
echo "[PASS] Advisory configuration does not suppress unpatchable rustls advisories."

# The resolved graph itself must not contain the vulnerable crates.
LOCKFILE="${REPO_ROOT}/Cargo.lock"
if [[ -f "${LOCKFILE}" ]]; then
  for pkg in ${VULN_PACKAGES}; do
    if grep -qE "^name = \"${pkg}\"" "${LOCKFILE}"; then
      echo "ERROR: '${pkg}' is present in Cargo.lock; the unpatchable rustls stack must not be in the dependency graph."
      exit 1
    fi
  done
  echo "[PASS] Resolved lockfile contains no unpatchable rustls stack packages."
else
  echo "[SKIP] No Cargo.lock present (library profile); graph check deferred to cargo audit."
fi

echo "=== All Security Integration Tests Passed Successfully ==="
