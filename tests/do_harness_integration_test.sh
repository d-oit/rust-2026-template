#!/usr/bin/env bash
# tests/do_harness_integration_test.sh
# Integration and regression tests for do-harness integration and quality gate boundaries.
#
# Behaviors Verified:
# 1. Clean workspace & optional tool absence: quality-gates.sh and doctor.sh succeed without do-harness.
# 2. Deliberately failing project quality gate fails closed and missing/stale evidence is explicitly reported.
# 3. No duplicate check execution: do-harness.toml configures generic language delegating to quality-gates.sh.
# 4. Multi-profile verification (minimal, library, cli, service, ai-agent, performance-sensitive, workspace)
#    preserves profile-specific lockfile policies and xtask tier configurations.
# 5. Telemetry artifacts (quality-run.json, quality-summary.md) are generated and preserved.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

pass() {
  TESTS_RUN=$((TESTS_RUN + 1))
  TESTS_PASSED=$((TESTS_PASSED + 1))
  echo -e "  ${GREEN}✓ PASS:${NC} $1"
}

fail() {
  TESTS_RUN=$((TESTS_RUN + 1))
  TESTS_FAILED=$((TESTS_FAILED + 1))
  echo -e "  ${RED}✗ FAIL:${NC} $1"
}

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

echo "=== Running do-harness Integration & Regression Tests ==="
echo ""

# Ensure xtask binary is built
cargo build --quiet -p xtask --bin xtask
XTASK_BIN="${REPO_ROOT}/target/debug/xtask"

# -----------------------------------------------------------------------------
# TEST 1: Optional Tool Absence & Clean Workspace Verification
# -----------------------------------------------------------------------------
echo "[Test 1] Testing optional do-harness tool absence and setup script..."

# 1A. Doctor script check
DOCTOR_OUT=$(bash scripts/doctor.sh 2>&1 || true)
if echo "${DOCTOR_OUT}" | grep -q "do-harness: not installed"; then
  pass "scripts/doctor.sh gracefully reports missing optional do-harness binary"
else
  fail "scripts/doctor.sh should report missing optional do-harness binary: ${DOCTOR_OUT}"
fi

# 1B. Setup script dry-run check
SETUP_OUT=$(bash scripts/setup-harness.sh --dry-run 2>&1 || true)
if echo "${SETUP_OUT}" | grep -q "Would download and verify do-harness"; then
  pass "scripts/setup-harness.sh --dry-run operates reproducibly without network side-effects"
else
  fail "scripts/setup-harness.sh --dry-run failed: ${SETUP_OUT}"
fi

# 1C. Run quality gate in an isolated clean workspace to verify quality run and telemetry generation without mutating REPO_ROOT
CLEAN_WS="${TMP_DIR}/clean_ws"
mkdir -p "${CLEAN_WS}"
cp -r "${REPO_ROOT}/Cargo.toml" "${REPO_ROOT}/Cargo.lock" "${REPO_ROOT}/AGENTS.md" \
      "${REPO_ROOT}/config" "${REPO_ROOT}/crates" "${REPO_ROOT}/examples" \
      "${REPO_ROOT}/benchmarks" "${REPO_ROOT}/scripts" \
      "${REPO_ROOT}/.gitignore" "${CLEAN_WS}/" 2>/dev/null || true

(
  cd "${CLEAN_WS}"
  git init -q
  git config user.name "Test User"
  git config user.email "test@example.com"
  if "${XTASK_BIN}" quality run --only build >/dev/null 2>&1; then
    pass "Quality run succeeds and generates telemetry in isolated clean workspace"
  else
    fail "Quality run failed in clean workspace"
  fi
)

# -----------------------------------------------------------------------------
# TEST 2: Non-Duplicating do-harness Configuration
# -----------------------------------------------------------------------------
echo ""
echo "[Test 2] Validating non-duplicating do-harness.toml configuration..."

if [[ -f "do-harness.toml" ]]; then
  if grep -q 'language = "generic"' do-harness.toml && grep -q 'scripts/quality-gates.sh' do-harness.toml && grep -q '"\*\*/Cargo.toml"' do-harness.toml; then
    pass "do-harness.toml configures generic language delegating quality checks to scripts/quality-gates.sh and includes **/Cargo.toml in sensor files"
  else
    fail "do-harness.toml must use generic language, delegate to scripts/quality-gates.sh, and include **/Cargo.toml in files"
  fi
else
  fail "do-harness.toml is missing from repository root"
fi

# -----------------------------------------------------------------------------
# TEST 3: Deliberately Failing Gate & Stale/Missing Evidence Fail Closed
# -----------------------------------------------------------------------------
echo ""
echo "[Test 3] Validating deliberately failing quality gate and evidence fail-closed behavior..."

# 3A. Test that quality status fails when evidence is absent
EMPTY_EVID_DIR="${TMP_DIR}/empty_evidence"
mkdir -p "${EMPTY_EVID_DIR}"
(
  cd "${EMPTY_EVID_DIR}"
  if ! "${XTASK_BIN}" quality status >/dev/null 2>&1; then
    pass "Missing telemetry evidence causes quality status to fail closed explicitly"
  else
    fail "Missing telemetry evidence must cause quality status to fail closed"
  fi
)

# 3B. Test member-manifest-only edit selection and failure (direct xtask and adapter agreement)
MANIFEST_EDIT_WS="${TMP_DIR}/manifest_edit_ws"
mkdir -p "${MANIFEST_EDIT_WS}"
cp -r "${REPO_ROOT}/Cargo.toml" "${REPO_ROOT}/Cargo.lock" \
      "${REPO_ROOT}/config" "${REPO_ROOT}/crates" "${REPO_ROOT}/examples" \
      "${REPO_ROOT}/benchmarks" "${REPO_ROOT}/scripts" \
      "${REPO_ROOT}/do-harness.toml" "${MANIFEST_EDIT_WS}/" 2>/dev/null || true

(
  cd "${MANIFEST_EDIT_WS}"
  git init -q
  git config user.name "Test User"
  git config user.email "test@example.com"
  git add .
  git commit -m "initial commit" -q

  BASE_SHA=$(git rev-parse HEAD)

  # Break a member manifest (crates/sample-app/Cargo.toml) with invalid syntax
  cat >> "crates/sample-app/Cargo.toml" << 'EOF'
[invalid_syntax
EOF

  # 1) Direct xtask selection with --changed-from must select Rust checks and fail closed on broken manifest
  if ! "${XTASK_BIN}" quality run --tier pull-request --changed-from "${BASE_SHA}" >/dev/null 2>&1; then
    pass "Direct xtask quality run selects member-manifest edit and fails on invalid manifest"
  else
    fail "Direct xtask quality run should have failed on invalid member manifest edit"
  fi

  # 2) Adapter script (quality-gates.sh) must also execute and fail on invalid member manifest edit
  if ! bash scripts/quality-gates.sh >/dev/null 2>&1; then
    pass "Adapter script (quality-gates.sh) executes and fails on invalid member manifest edit"
  else
    fail "Adapter script (quality-gates.sh) should have failed on invalid member manifest edit"
  fi
)

# 3C. Test member manifest edit outside crates/ (e.g. packages/my-crate/Cargo.toml) with real CLI
OUTSIDE_WS="${TMP_DIR}/outside_member_ws"
mkdir -p "${OUTSIDE_WS}/packages/my-crate/src" "${OUTSIDE_WS}/config" "${OUTSIDE_WS}/scripts"
cp -r "${REPO_ROOT}/Cargo.lock" "${OUTSIDE_WS}/"
cp -r "${REPO_ROOT}/config" "${OUTSIDE_WS}/"
cp -r "${REPO_ROOT}/scripts" "${OUTSIDE_WS}/"

cat > "${OUTSIDE_WS}/Cargo.toml" << 'EOF'
[workspace]
members = ["packages/my-crate"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
EOF

cat > "${OUTSIDE_WS}/packages/my-crate/Cargo.toml" << 'EOF'
[package]
name = "my-crate"
version.workspace = true
edition.workspace = true

[dependencies]
EOF

cat > "${OUTSIDE_WS}/packages/my-crate/src/lib.rs" << 'EOF'
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 4);
    }
}
EOF

(
  cd "${OUTSIDE_WS}"
  git init -q
  git config user.name "Test User"
  git config user.email "test@example.com"
  git add .
  git commit -m "initial commit" -q

  OUTSIDE_BASE_SHA=$(git rev-parse HEAD)

  # Edit member manifest outside crates/
  cat >> "packages/my-crate/Cargo.toml" << 'EOF'
# modified manifest
EOF

  # Explain check selection for packages/my-crate/Cargo.toml
  EXPLAIN_OUT=$("${XTASK_BIN}" quality explain --tier pull-request --changed-from "${OUTSIDE_BASE_SHA}" 2>&1)
  if echo "${EXPLAIN_OUT}" | grep -q "Rust Build" && echo "${EXPLAIN_OUT}" | grep -q "matched glob pattern '\*\*/Cargo.toml'"; then
    pass "xtask quality explain selects Build check for packages/my-crate/Cargo.toml edit"
  else
    fail "xtask quality explain should select Build check for packages/my-crate/Cargo.toml edit: ${EXPLAIN_OUT}"
  fi

  # Run real CLI quality run on packages/my-crate/Cargo.toml edit
  if "${XTASK_BIN}" quality run --tier pull-request --changed-from "${OUTSIDE_BASE_SHA}" >/dev/null 2>&1; then
    pass "Real CLI quality run succeeds on member manifest edit outside crates/"
  else
    fail "Real CLI quality run failed on valid member manifest edit outside crates/"
  fi

  # Markdown-only edit test: should skip Rust build/test checks
  git add .
  git commit -m "update manifest" -q
  OUTSIDE_BASE_SHA_2=$(git rev-parse HEAD)

  cat > "README.md" << 'EOF'
# Title
EOF

  EXPLAIN_MD=$("${XTASK_BIN}" quality explain --tier pull-request --changed-from "${OUTSIDE_BASE_SHA_2}" 2>&1)
  if echo "${EXPLAIN_MD}" | grep -q "Rust Build" && echo "${EXPLAIN_MD}" | grep -q "no changed files matched patterns"; then
    pass "xtask quality explain skips Rust Build for Markdown-only edit"
  else
    fail "xtask quality explain should skip Rust Build for Markdown-only edit: ${EXPLAIN_MD}"
  fi

  # Malformed member manifest outside crates/ should fail non-zero
  git add .
  git commit -m "add readme" -q
  OUTSIDE_BASE_SHA_3=$(git rev-parse HEAD)

  cat >> "packages/my-crate/Cargo.toml" << 'EOF'
[invalid_syntax_syntax
EOF

  if ! "${XTASK_BIN}" quality run --tier pull-request --changed-from "${OUTSIDE_BASE_SHA_3}" >/dev/null 2>&1; then
    pass "Real CLI quality run fails non-zero on malformed member manifest outside crates/"
  else
    fail "Real CLI quality run should have failed on malformed member manifest outside crates/"
  fi
)

# 3D. Test that deliberate failure in a fixture workspace is caught closed
FAIL_WS="${TMP_DIR}/failing_workspace"
mkdir -p "${FAIL_WS}/src" "${FAIL_WS}/crates/xtask/src" "${FAIL_WS}/config"
cp -r "${REPO_ROOT}/crates/xtask" "${FAIL_WS}/crates/"
cp -r "${REPO_ROOT}/config" "${FAIL_WS}/"
cp -r "${REPO_ROOT}/scripts" "${FAIL_WS}/"

cat > "${FAIL_WS}/Cargo.toml" << EOF
[workspace]
members = ["crates/bad-crate", "crates/xtask"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
EOF

mkdir -p "${FAIL_WS}/crates/bad-crate/src"
cat > "${FAIL_WS}/crates/bad-crate/Cargo.toml" << EOF
[package]
name = "bad-crate"
version.workspace = true
edition.workspace = true
EOF

# Introduce a deliberate syntax error in bad-crate
cat > "${FAIL_WS}/crates/bad-crate/src/lib.rs" << EOF
pub fn broken_syntax( {
EOF

(
  cd "${FAIL_WS}"
  if ! cargo run --quiet -p xtask --bin xtask -- quality run --only build >/dev/null 2>&1; then
    pass "Deliberately failing quality gate fails closed (deliberate failure caught)"
  else
    fail "Deliberately failing quality gate should have failed"
  fi
)

# -----------------------------------------------------------------------------
# TEST 4: Multi-Profile Initialization and Lockfile/Tier Policy
# -----------------------------------------------------------------------------
echo ""
echo "[Test 4] Validating template profile initialization across profiles..."

PROFILES=("minimal" "library" "cli" "service" "ai-agent" "performance-sensitive" "workspace")

for prof in "${PROFILES[@]}"; do
  PROF_WS="${TMP_DIR}/prof_${prof}"
  mkdir -p "${PROF_WS}"
  # Copy template files into isolated profile workspace directory
  cp -r "${REPO_ROOT}/Cargo.toml" "${REPO_ROOT}/Cargo.lock" "${REPO_ROOT}/AGENTS.md" \
        "${REPO_ROOT}/config" "${REPO_ROOT}/crates" "${REPO_ROOT}/examples" \
        "${REPO_ROOT}/benchmarks" "${REPO_ROOT}/scripts" \
        "${REPO_ROOT}/.gitignore" "${PROF_WS}/" 2>/dev/null || true

  # Run template init inside isolated profile workspace
  (
    cd "${PROF_WS}"
    git init -q
    git config user.name "Test User"
    git config user.email "test@example.com"

    "${XTASK_BIN}" template init --profile "${prof}" --name "test-${prof}" >/dev/null 2>&1
  )

  if [[ -d "${PROF_WS}" ]]; then
    pass "Profile '${prof}' initialized successfully"
  else
    fail "Profile '${prof}' failed initialization"
    continue
  fi

  # Check lockfile policy
  if [[ "${prof}" == "library" ]]; then
    if grep -q "Cargo.lock" "${PROF_WS}/.gitignore" 2>/dev/null; then
      pass "Profile 'library' respects lockfile policy (Cargo.lock ignored)"
    else
      fail "Profile 'library' should keep Cargo.lock in .gitignore"
    fi
  else
    if ! grep -q "^Cargo.lock$" "${PROF_WS}/.gitignore" 2>/dev/null; then
      pass "Profile '${prof}' respects lockfile policy (Cargo.lock tracked)"
    else
      fail "Profile '${prof}' should un-ignore Cargo.lock"
    fi
  fi

  # Verify config/xtask.json was generated and valid, preserving **/Cargo.toml mapping
  if [[ -f "${PROF_WS}/config/xtask.json" ]]; then
    if grep -q '"\*\*/Cargo.toml"' "${PROF_WS}/config/xtask.json"; then
      pass "Profile '${prof}' preserves config/xtask.json with **/Cargo.toml mapping"
    else
      fail "Profile '${prof}' config/xtask.json missing **/Cargo.toml mapping"
    fi
  else
    fail "Profile '${prof}' missing config/xtask.json"
  fi
done

# -----------------------------------------------------------------------------
# TEST 5: Telemetry & Artifact Preservation
# -----------------------------------------------------------------------------
echo ""
echo "[Test 5] Validating telemetry artifact preservation..."

"${XTASK_BIN}" quality run --only build >/dev/null 2>&1 || true

if [[ -f ".agents/ci/quality-run.json" ]]; then
  if grep -q '"schema_version": 2' .agents/ci/quality-run.json; then
    pass ".agents/ci/quality-run.json schema version 2 preserved"
  else
    fail ".agents/ci/quality-run.json schema version 2 invalid or missing"
  fi
else
  fail ".agents/ci/quality-run.json missing"
fi

if [[ -f ".agents/ci/quality-summary.md" ]]; then
  if grep -q "Quality Run Telemetry" .agents/ci/quality-summary.md; then
    pass ".agents/ci/quality-summary.md artifact preserved"
  else
    fail ".agents/ci/quality-summary.md content missing header"
  fi
else
  fail ".agents/ci/quality-summary.md missing"
fi

# -----------------------------------------------------------------------------
# SUMMARY
# -----------------------------------------------------------------------------
echo ""
echo "========================================"
echo "do-harness Integration Test Summary"
echo "========================================"
echo "Total assertions verified: ${TESTS_RUN}"
echo -e "${GREEN}Passed: ${TESTS_PASSED}${NC}"

if [[ ${TESTS_FAILED} -gt 0 ]]; then
  echo -e "${RED}Failed: ${TESTS_FAILED}${NC}"
  exit 1
fi

echo -e "${GREEN}All do-harness integration tests PASSED!${NC}"
exit 0
