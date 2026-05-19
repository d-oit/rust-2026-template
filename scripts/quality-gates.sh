#!/usr/bin/env bash
# scripts/quality-gates.sh
set -euo pipefail
# Run all quality gates locally - mirrors CI pipeline
# Usage: ./scripts/quality-gates.sh [--fix]

FIX=false
for arg in "$@"; do
  case $arg in
    --fix) FIX=true ;;
    *) echo "Unknown argument: $arg"; exit 1 ;;
  esac
done

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

pass() { echo -e "${GREEN}[PASS]${NC} $1"; }
fail() { echo -e "${RED}[FAIL]${NC} $1"; exit 1; }
info() { echo -e "${YELLOW}[INFO]${NC} $1"; }

# Parse .test-quality.toml for thresholds
TOML_FILE=".test-quality.toml"
if [ -f "$TOML_FILE" ]; then
  MIN_TEST_COUNT=$(python3 -c "import toml; print(toml.load('$TOML_FILE')['tests']['min_test_count'])")
  MIN_TEST_TO_SOURCE_RATIO=$(python3 -c "import toml; print(toml.load('$TOML_FILE')['ratios']['min_test_to_source_ratio'])")
  MAX_FILE_LOC=$(python3 -c "import toml; print(toml.load('$TOML_FILE')['tests']['max_file_loc'])")
else
  info "No .test-quality.toml found, using defaults"
  MIN_TEST_COUNT=20
  MIN_TEST_TO_SOURCE_RATIO=0.5
  MAX_FILE_LOC=500
fi

info "Starting quality gates..."
info "Test quality thresholds: min_tests=$MIN_TEST_COUNT, min_ratio=$MIN_TEST_TO_SOURCE_RATIO, max_file_loc=$MAX_FILE_LOC"

# ============================================================
# 1. FORMAT CHECK
# ============================================================
info "[1/9] Checking formatting..."
if $FIX; then
  cargo fmt --all
  pass "Format: auto-fixed"
else
  cargo fmt --all -- --check || fail "Format: run 'cargo fmt --all' to fix"
  pass "Format: OK"
fi

# ============================================================
# 2. CLIPPY
# ============================================================
info "[2/9] Running Clippy..."
if $FIX; then
  cargo clippy --fix --allow-dirty --allow-staged --all-targets --all-features
  pass "Clippy: auto-fixed"
else
  cargo clippy --all-targets --all-features -- -D warnings || fail "Clippy: fix lint errors above"
  pass "Clippy: OK"
fi

# ============================================================
# 3. BUILD
# ============================================================
info "[3/9] Building..."
cargo build --all-targets || fail "Build: failed"
pass "Build: OK"

# ============================================================
# 4. TESTS
# ============================================================
info "[4/9] Running tests..."
if command -v cargo-nextest &>/dev/null; then
  cargo nextest run --all-features --workspace || fail "Tests: failed"
else
  info "cargo-nextest not found, using cargo test"
  cargo test --all-features --workspace || fail "Tests: failed"
fi

# Doc tests
cargo test --doc --all-features || fail "Doc tests: failed"
pass "Tests: OK"

# ============================================================
# 5. SECURITY AUDIT
# ============================================================
info "[5/9] Security audit..."
if command -v cargo-audit &>/dev/null; then
  cargo audit || fail "Security audit: vulnerabilities found"
  pass "Audit: OK"
else
  info "cargo-audit not installed, skipping (run: cargo install cargo-audit)"
fi

# ============================================================
# 6. SUPPLY CHAIN (cargo-deny)
# ============================================================
info "[6/9] Supply chain check..."
if command -v cargo-deny &>/dev/null; then
  cargo deny check || fail "cargo-deny: violations found"
  pass "Deny: OK"
else
  info "cargo-deny not installed, skipping (run: cargo install cargo-deny)"
fi

# ============================================================
# 7. UNUSED DEPENDENCIES
# ============================================================
info "[7/9] Checking unused dependencies..."
if command -v cargo-machete &>/dev/null; then
  cargo machete || fail "Unused deps found"
  pass "Machete: OK"
else
  info "cargo-machete not installed, skipping (run: cargo install cargo-machete)"
fi

# ============================================================
# 8. PRIVACY CHECK (No emails)
# ============================================================
info "[8/9] Checking for email addresses (privacy-first)..."
EMAIL_PATTERN='[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}'
EXCLUDE_PATTERN='example\.com|example\.org|test\.com|\.git|target'

# ⚡ Bolt: Optimized by using --exclude-dir to skip large/irrelevant directories
# instead of filtering results after a full recursive scan. This significantly
# reduces I/O and CPU time in large Rust projects with deep target/ folders.
# Note: .agents IS included as it contains critical workflow definitions.
if grep -rE "$EMAIL_PATTERN" \
  --exclude-dir=.git --exclude-dir=target \
  . 2>/dev/null | grep -vE "$EXCLUDE_PATTERN"; then
  fail "Email address detected in codebase. Please remove it to comply with privacy-first policy."
else
  pass "Privacy: OK"
fi

# ============================================================
# 9. SECRET SCAN
# ============================================================
info "[9/9] Scanning for potential secrets..."
# Matches patterns like api_key = "..." with at least 16 characters in the secret
SECRET_PATTERN="(api_key|token|secret|password|auth|key)[[:space:]]*[:=][[:space:]]*['\"][a-zA-Z0-9_\-]{16,}['\"]"
EXCLUDE_DIR='--exclude-dir=.git --exclude-dir=target --exclude-dir=.agents'
EXCLUDE_SECRET='example\.com|example\.org|test\.com|GITHUB_TOKEN|CARGO_REGISTRY_TOKEN'

if grep -rE $EXCLUDE_DIR "$SECRET_PATTERN" . 2>/dev/null | grep -vE "$EXCLUDE_SECRET"; then
  fail "Potential secret detected in codebase. Please use environment variables instead."
else
  pass "Secret Scan: OK"
fi

# ============================================================
# 10. TEST COUNT CHECK
# ============================================================
info "[10/10] Checking test count..."
if command -v rg &>/dev/null; then
  # Count test functions across src/, crates/, and tests/ directories
  TEST_COUNT=$(rg -c '#\[test\]|#\[tokio::test\]' src/ crates/ tests/ 2>/dev/null | \
    awk -F: '{sum += $2} END {print sum+0}')
  
  if [ "$TEST_COUNT" -lt "$MIN_TEST_COUNT" ]; then
    fail "Test count $TEST_COUNT below minimum $MIN_TEST_COUNT (defined in $TOML_FILE)"
  else
    pass "Test count: $TEST_COUNT >= $MIN_TEST_COUNT"
  fi
else
  info "ripgrep (rg) not installed, skipping test count check"
fi

# ============================================================
# 11. FILE SIZE CHECK
# ============================================================
info "[11/11] Checking file sizes..."
OVERSIZED_FILES=$(find src/ crates/ -name "*.rs" -type f -print0 2>/dev/null | \
  xargs -0 wc -l 2>/dev/null | grep -v "total$" | \
  awk -v max="$MAX_FILE_LOC" '$1 > max {print $2 " (" $1 " LOC)"}')

if [ -n "$OVERSIZED_FILES" ]; then
  fail "Oversized files found (max $MAX_FILE_LOC LOC):\n$OVERSIZED_FILES"
else
  pass "File sizes: All files <= $MAX_FILE_LOC LOC"
fi

echo ""
echo -e "${GREEN}All quality gates passed!${NC}"
