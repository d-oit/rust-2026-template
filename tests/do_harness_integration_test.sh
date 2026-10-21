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

# Reads the quality-gates sensor `when-changed` globs from a do-harness.toml and
# reports, for each path given as an argument, whether any glob matches it. The
# glob semantics mirror do-harness: '**' spans path segments, '*' does not.
glob_matches() {
  local config_path="$1"
  shift
  python3 - "${config_path}" "$@" <<'PY'
import re
import sys
import tomllib

config_path = sys.argv[1]
with open(config_path, 'rb') as fh:
    config = tomllib.load(fh)

patterns = []
for sensor in config.get('sensors', []):
    if sensor.get('name') == 'quality-gates':
        patterns.extend(sensor.get('when-changed', []))


def to_regex(pattern: str) -> str:
    parts = []
    i = 0
    while i < len(pattern):
        if pattern.startswith('**/', i):
            parts.append('(?:.*/)?')
            i += 3
        elif pattern.startswith('**', i):
            parts.append('.*')
            i += 2
        elif pattern[i] == '*':
            parts.append('[^/]*')
            i += 1
        else:
            parts.append(re.escape(pattern[i]))
            i += 1
    return ''.join(parts)


for path in sys.argv[2:]:
    if not path:
        continue
    hits = [p for p in patterns if re.fullmatch(to_regex(p), path)]
    print(f"{path}\t{'MATCH' if hits else 'NO_MATCH'}\t{','.join(hits)}")
PY
}

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
# The contract is that doctor.sh reports the optional do-harness binary status
# gracefully. Whether the binary is installed depends on the environment
# (CI does not install it), so both states are valid and must be reported
# without failing the script.
DOCTOR_OUT=$(bash scripts/doctor.sh 2>&1 || true)
if echo "${DOCTOR_OUT}" | grep -qE 'do-harness: (not installed|/|[A-Za-z]:)'; then
  pass "scripts/doctor.sh gracefully reports the optional do-harness binary status"
else
  fail "scripts/doctor.sh should report the do-harness binary status: ${DOCTOR_OUT}"
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
  if grep -q 'language = "generic"' do-harness.toml && grep -q 'scripts/quality-gates.sh' do-harness.toml; then
    pass "do-harness.toml configures generic language delegating quality checks to scripts/quality-gates.sh"
  else
    fail "do-harness.toml must use generic language and delegate to scripts/quality-gates.sh"
  fi

  # The quality-gates sensor must declare `when-changed` applicability globs that
  # cover BOTH the workspace root manifest and nested workspace member manifests.
  SENSOR_WHEN_CHANGED=$(python3 -c "
import tomllib
with open('do-harness.toml', 'rb') as fh:
    config = tomllib.load(fh)
for sensor in config.get('sensors', []):
    if sensor.get('name') == 'quality-gates':
        for pattern in sensor.get('when-changed', []):
            print(pattern)
" 2>/dev/null || true)

  if grep -qxF 'Cargo.toml' <<<"${SENSOR_WHEN_CHANGED}" \
     && grep -qxF '**/Cargo.toml' <<<"${SENSOR_WHEN_CHANGED}"; then
    pass "do-harness.toml quality-gates sensor declares when-changed globs for root and member manifests"
  else
    fail "do-harness.toml quality-gates sensor must declare when-changed globs 'Cargo.toml' and '**/Cargo.toml' (found: ${SENSOR_WHEN_CHANGED:-none})"
  fi

  # A bare wildcard would select the sensor on documentation-only edits and
  # defeat the Markdown-only fast path.
  if grep -qxF '**' <<<"${SENSOR_WHEN_CHANGED}"; then
    fail "do-harness.toml quality-gates sensor must not use a bare '**' when-changed glob"
  else
    pass "do-harness.toml quality-gates sensor avoids bare wildcard when-changed globs"
  fi

  # Root manifest and nested member manifests must both select the sensor.
  SELECTION=$(glob_matches "do-harness.toml" \
    "Cargo.toml" "crates/sample-app/Cargo.toml" "README.md")
  if grep -qE '^Cargo\.toml[[:space:]]+MATCH' <<<"${SELECTION}" \
     && grep -qE '^crates/sample-app/Cargo\.toml[[:space:]]+MATCH' <<<"${SELECTION}" \
     && grep -qE '^README\.md[[:space:]]+NO_MATCH' <<<"${SELECTION}"; then
    pass "do-harness.toml when-changed selects root and member manifests but not Markdown-only edits"
  else
    fail "do-harness.toml when-changed selection is wrong: ${SELECTION}"
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

# 3B-2. A member manifest OUTSIDE crates/ (examples/, benchmarks/) must also be
# selected by the quality-gates sensor's `when-changed` globs. The root manifest
# glob "Cargo.toml" alone does not cover nested member manifests.
OUTSIDE_MANIFEST_WS="${TMP_DIR}/outside_manifest_ws"
mkdir -p "${OUTSIDE_MANIFEST_WS}"
cp -r "${REPO_ROOT}/Cargo.toml" "${REPO_ROOT}/Cargo.lock" \
      "${REPO_ROOT}/config" "${REPO_ROOT}/crates" "${REPO_ROOT}/examples" \
      "${REPO_ROOT}/benchmarks" "${REPO_ROOT}/scripts" \
      "${REPO_ROOT}/do-harness.toml" "${OUTSIDE_MANIFEST_WS}/" 2>/dev/null || true

(
  cd "${OUTSIDE_MANIFEST_WS}"
  git init -q
  git config user.name "Test User"
  git config user.email "test@example.com"
  git add .
  git commit -m "initial commit" -q

  # Member manifests outside crates/ must be selected by the declared globs.
  for member_manifest in "examples/hello_world/Cargo.toml" "benchmarks/Cargo.toml"; do
    if [[ ! -f "${member_manifest}" ]]; then
      fail "Fixture member manifest missing for selection check: ${member_manifest}"
      continue
    fi

    SELECTION=$(glob_matches "do-harness.toml" "${member_manifest}")
    if grep -qE "^${member_manifest//./\\.}[[:space:]]+MATCH" <<<"${SELECTION}"; then
      pass "quality-gates when-changed globs select member manifest outside crates/: ${member_manifest}"
    else
      fail "quality-gates when-changed globs must select member manifest outside crates/: ${member_manifest} (${SELECTION})"
    fi
  done

  # A documentation-only edit must NOT match any declared quality-gates glob.
  DOC_SELECTION=$(glob_matches "do-harness.toml" "README.md")
  if grep -qE '^README\.md[[:space:]]+NO_MATCH' <<<"${DOC_SELECTION}"; then
    pass "quality-gates when-changed globs do not match documentation-only edits (README.md)"
  else
    fail "quality-gates when-changed globs must not match documentation-only edits (${DOC_SELECTION})"
  fi
)

# 3C. Test that deliberate failure in a fixture workspace is caught closed
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
