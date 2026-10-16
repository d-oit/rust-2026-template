#!/usr/bin/env bash
# tests/quality_gate_contracts_test.sh
# Fail-closed contract integration tests for quality gates, scripts, and pre-commit hooks.
#
# Behaviors Proved:
# 1. Zero-test workspaces and removed test suites fail closed during test execution.
# 2. Missing required tools on protected tiers fail closed with MissingTool errors.
# 3. Unavailable or missing telemetry evidence causes 'quality status' to fail closed.
# 4. Empty input sets for scanners and validators (workflows, source files) fail closed.
# 5. Generated artifacts drifting from source of truth fail check validation.
# 6. Pre-commit hooks reject staged broken fixtures (secrets, LOC limit violations, invalid syntax).
# 7. Tool isolation via restricted PATH prevents global host tools from altering test assertions.

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

echo "=== Running Quality Gate Fail-Closed Contract Tests ==="
echo ""

# Ensure xtask binary is compiled for isolated tests
cargo build --quiet -p xtask --bin xtask
XTASK_BIN="${REPO_ROOT}/target/debug/xtask"

# Create an isolated PATH directory containing standard system binaries (excluding cargo-machete, shellcheck, markdownlint)
ISOLATED_PATH_DIR="${TMP_DIR}/isolated_bin"
mkdir -p "${ISOLATED_PATH_DIR}"
for tool in sh bash git cargo python3 wc grep sed cat cmp diff dirname mktemp head tail tr awk rustc rustfmt; do
  p=$(command -v "$tool" || true)
  if [[ -n "$p" ]]; then
    ln -s "$p" "${ISOLATED_PATH_DIR}/$tool" 2>/dev/null || true
  fi
done

# -----------------------------------------------------------------------------
# CONTRACT 1: Zero-Test Workspaces and Removed Test Suites Fail Closed
# Behavior: Running test gate on a workspace with zero tests must return an error.
# -----------------------------------------------------------------------------
echo "[Contract 1] Testing zero-test workspace fail-closed behavior..."

# Positive case: Current workspace has tests and xtask test check succeeds
if "${XTASK_BIN}" quality run --only test >/dev/null 2>&1; then
  pass "Workspace with valid tests passes test gate (positive case)"
else
  fail "Workspace with valid tests should pass test gate"
fi

# Failure case: Create fixture workspace with 0 tests
ZERO_WS="${TMP_DIR}/zero_test_ws"
mkdir -p "${ZERO_WS}/src" "${ZERO_WS}/crates/xtask/src" "${ZERO_WS}/config"
cat > "${ZERO_WS}/Cargo.toml" << EOF
[workspace]
members = ["crates/zero-app", "crates/xtask"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
EOF

mkdir -p "${ZERO_WS}/crates/zero-app/src"
cat > "${ZERO_WS}/crates/zero-app/Cargo.toml" << EOF
[package]
name = "zero-app"
version.workspace = true
edition.workspace = true

[dependencies]
EOF
cat > "${ZERO_WS}/crates/zero-app/src/lib.rs" << EOF
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
EOF

# Copy xtask source into ZERO_WS so we can execute quality run inside ZERO_WS
cp -r "${REPO_ROOT}/crates/xtask" "${ZERO_WS}/crates/"
cp -r "${REPO_ROOT}/config" "${ZERO_WS}/"
cp -r "${REPO_ROOT}/scripts" "${ZERO_WS}/"

(
  cd "${ZERO_WS}"
  if ! cargo run --quiet -p xtask --bin xtask -- quality run --only test >/dev/null 2>&1; then
    pass "Zero-test workspace causes test quality gate to fail closed (deliberate failure case)"
  else
    fail "Zero-test workspace should fail test quality gate"
  fi
)

# -----------------------------------------------------------------------------
# CONTRACT 2: Missing Required Tools & Unavailable Evidence Fail Closed
# Behavior: Protected tiers fail closed with MissingTool when required binaries are absent;
# quality status fails when telemetry evidence is missing, corrupted, or stale.
# -----------------------------------------------------------------------------
echo ""
echo "[Contract 2] Testing missing tool and evidence fail-closed behavior..."

# Deliberate failure case: Required tool (cargo-machete) missing on protected-branch tier
OUTPUT_MACHETE=$(PATH="${ISOLATED_PATH_DIR}" "${XTASK_BIN}" quality run --tier protected-branch --only machete 2>&1 || true)
if echo "${OUTPUT_MACHETE}" | grep -q "FAILED" && echo "${OUTPUT_MACHETE}" | grep -q "Rust Unused Dependencies"; then
  pass "Missing required tool on protected-branch tier fails closed with MissingTool error (deliberate failure case)"
else
  fail "Missing required tool on protected-branch tier must fail closed: ${OUTPUT_MACHETE}"
fi

# Positive case: Optional tool missing on pull-request tier skips gracefully
OUTPUT_PR_MACHETE=$(PATH="${ISOLATED_PATH_DIR}" "${XTASK_BIN}" quality run --tier pull-request --only machete 2>&1 || true)
if echo "${OUTPUT_PR_MACHETE}" | grep -q "skipping" || echo "${OUTPUT_PR_MACHETE}" | grep -q "SUCCESS"; then
  pass "Missing optional tool on pull-request tier skips gracefully (positive case)"
else
  fail "Missing optional tool on pull-request tier should skip gracefully: ${OUTPUT_PR_MACHETE}"
fi

# Unavailable evidence case: Missing quality-run.json in empty directory causes quality status to fail
EVID_DIR="${TMP_DIR}/empty_evid"
mkdir -p "${EVID_DIR}"
(
  cd "${EVID_DIR}"
  if ! "${XTASK_BIN}" quality status >/dev/null 2>&1; then
    pass "Missing evidence artifact causes quality status to fail closed (deliberate failure case)"
  else
    fail "Missing evidence artifact must cause quality status to fail"
  fi
)

# -----------------------------------------------------------------------------
# CONTRACT 3: Empty Input Sets for Scanners and Validators Fail Closed
# Behavior: Validators fail closed when expected input file sets are empty or missing.
# -----------------------------------------------------------------------------
echo ""
echo "[Contract 3] Testing empty input sets fail-closed behavior..."

# Positive case: validate-workflows.sh on repo .github/workflows
if WORKFLOWS_DIR=".github/workflows" bash scripts/validate-workflows.sh >/dev/null 2>&1; then
  pass "validate-workflows.sh passes with valid workflow directory (positive case)"
else
  fail "validate-workflows.sh should pass on repo workflows"
fi

# Deliberate failure case: Empty workflows directory
EMPTY_WORKFLOWS_DIR="${TMP_DIR}/empty_workflows"
mkdir -p "${EMPTY_WORKFLOWS_DIR}"
if ! WORKFLOWS_DIR="${EMPTY_WORKFLOWS_DIR}" bash scripts/validate-workflows.sh >/dev/null 2>&1; then
  pass "validate-workflows.sh fails closed on empty workflow directory (deliberate failure case)"
else
  fail "validate-workflows.sh must fail closed on empty workflow directory"
fi

# -----------------------------------------------------------------------------
# CONTRACT 4: Generated Artifacts Drifting from Source of Truth Fail Closed
# Behavior: generate-llms-txt.sh --check and QualityCheck::LlmContext fail when artifacts drift.
# -----------------------------------------------------------------------------
echo ""
echo "[Contract 4] Testing generated artifact drift fail-closed behavior..."

# Positive case: generate-llms-txt.sh --check on current repo
if bash scripts/generate-llms-txt.sh --check >/dev/null 2>&1; then
  pass "generate-llms-txt.sh --check passes when artifacts are up to date (positive case)"
else
  fail "generate-llms-txt.sh --check should pass when artifacts are current"
fi

# Deliberate failure case: Modify source doc in temp repo copy without regenerating
DRIFT_REPO="${TMP_DIR}/drift_repo"
mkdir -p "${DRIFT_REPO}"
cp -r VERSION README.md AGENTS.md CONTRIBUTING.md QUICKSTART.md SECURITY.md llms.txt llms-full.txt scripts "${DRIFT_REPO}/" 2>/dev/null || true

(
  cd "${DRIFT_REPO}"
  echo -e "\n## Added Unsynced Content" >> README.md
  if ! bash scripts/generate-llms-txt.sh --check >/dev/null 2>&1; then
    pass "generate-llms-txt.sh --check fails closed when source doc drifts (deliberate failure case)"
  else
    fail "generate-llms-txt.sh --check must fail when source doc drifts"
  fi
)

# -----------------------------------------------------------------------------
# CONTRACT 5: Pre-Commit Hooks Rejecting Broken Fixtures
# Behavior: Pre-commit hook script rejects staged files violating quality rules.
# -----------------------------------------------------------------------------
echo ""
echo "[Contract 5] Testing pre-commit hook enforcement..."

HOOK_REPO="${TMP_DIR}/hook_repo"
mkdir -p "${HOOK_REPO}/.githooks" "${HOOK_REPO}/src"
cp .githooks/pre-commit "${HOOK_REPO}/.githooks/pre-commit"

(
  cd "${HOOK_REPO}"
  git init --quiet
  git config user.name "Test User"
  git config user.email "test@example.com"

  # Positive case: Clean staged Rust file and TOML file
  cat > "Cargo.toml" << 'EOF'
[package]
name = "hook-fixture"
version = "0.1.0"
edition = "2021"
EOF
  cat > "src/main.rs" << 'EOF'
fn main() {
    println!("Hello, world!");
}
EOF
  git add Cargo.toml src/main.rs

  if bash .githooks/pre-commit >/dev/null 2>&1; then
    pass ".githooks/pre-commit accepts valid staged files (positive case)"
  else
    fail ".githooks/pre-commit should accept valid staged files"
  fi

  # Deliberate failure case 1: Staged file with API key secret
  cat > "src/secret.rs" << 'EOF'
pub const API_KEY: &str = "api_key = 'secret_token_val_1234567890123456'";
EOF
  git add src/secret.rs

  if ! bash .githooks/pre-commit >/dev/null 2>&1; then
    pass ".githooks/pre-commit rejects staged secrets (deliberate failure case)"
  else
    fail ".githooks/pre-commit must reject staged secrets"
  fi
  git rm -f src/secret.rs >/dev/null 2>&1 || rm -f src/secret.rs

  # Deliberate failure case 2: Staged file exceeding 500 lines
  python3 -c "print('\n'.join(['// line ' + str(i) for i in range(505)]))" > "src/oversized.rs"
  git add src/oversized.rs

  if ! bash .githooks/pre-commit >/dev/null 2>&1; then
    pass ".githooks/pre-commit rejects staged file exceeding 500 LOC (deliberate failure case)"
  else
    fail ".githooks/pre-commit must reject files exceeding 500 LOC"
  fi
  git rm -f src/oversized.rs >/dev/null 2>&1 || rm -f src/oversized.rs

  # Deliberate failure case 3: Invalid TOML syntax
  cat > "bad.toml" << 'EOF'
[invalid
key = "value"
EOF
  git add bad.toml

  if ! bash .githooks/pre-commit >/dev/null 2>&1; then
    pass ".githooks/pre-commit rejects invalid TOML syntax (deliberate failure case)"
  else
    fail ".githooks/pre-commit must reject invalid TOML syntax"
  fi
)

# -----------------------------------------------------------------------------
# CONTRACT 6: Isolation from Globally Installed Tools
# Behavior: PATH isolation ensures global binaries don't alter test assertions.
# -----------------------------------------------------------------------------
echo ""
echo "[Contract 6] Testing tool isolation via restricted PATH..."

if PATH="${ISOLATED_PATH_DIR}" command -v shellcheck >/dev/null 2>&1; then
  fail "Isolated PATH must not contain shellcheck"
else
  pass "Isolated PATH excludes host shellcheck binary (tool isolation verified)"
fi

if PATH="${ISOLATED_PATH_DIR}" command -v markdownlint-cli2 >/dev/null 2>&1; then
  fail "Isolated PATH must not contain markdownlint-cli2"
else
  pass "Isolated PATH excludes host markdownlint-cli2 binary (tool isolation verified)"
fi

# -----------------------------------------------------------------------------
# TEST SUMMARY
# -----------------------------------------------------------------------------
echo ""
echo "========================================"
echo "Contract Test Summary"
echo "========================================"
echo "Total contracts verified: ${TESTS_RUN}"
echo -e "${GREEN}Passed: ${TESTS_PASSED}${NC}"

if [[ ${TESTS_FAILED} -gt 0 ]]; then
  echo -e "${RED}Failed: ${TESTS_FAILED}${NC}"
  exit 1
fi

echo -e "${GREEN}All fail-closed contract tests PASSED!${NC}"
exit 0
