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

info "Starting quality gates..."

# ============================================================
# 1. FORMAT CHECK
# ============================================================
info "[1/10] Checking formatting..."
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
info "[2/10] Running Clippy..."
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
info "[3/10] Building..."
cargo build --all-targets || fail "Build: failed"
pass "Build: OK"

# ============================================================
# 4. TESTS
# ============================================================
info "[4/10] Running tests..."
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
info "[5/10] Security audit..."
if command -v cargo-audit &>/dev/null; then
  cargo audit || fail "Security audit: vulnerabilities found"
  pass "Audit: OK"
else
  info "cargo-audit not installed, skipping (run: cargo install cargo-audit)"
fi

# ============================================================
# 6. SUPPLY CHAIN (cargo-deny)
# ============================================================
info "[6/10] Supply chain check..."
if command -v cargo-deny &>/dev/null; then
  cargo deny check || fail "cargo-deny: violations found"
  pass "Deny: OK"
else
  info "cargo-deny not installed, skipping (run: cargo install cargo-deny)"
fi

# ============================================================
# 7. UNUSED DEPENDENCIES
# ============================================================
info "[7/10] Checking unused dependencies..."
if command -v cargo-machete &>/dev/null; then
  cargo machete || fail "Unused deps found"
  pass "Machete: OK"
else
  info "cargo-machete not installed, skipping (run: cargo install cargo-machete)"
fi

# ============================================================
# 8. PRIVACY CHECK (No emails)
# ============================================================
info "[8/10] Checking for email addresses (privacy-first)..."
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
info "[9/10] Scanning for potential secrets..."
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
# 10. TEST QUALITY ENFORCEMENT
# ============================================================
info "[10/10] Checking test quality thresholds..."

if [ ! -f ".test-quality.toml" ]; then
  fail ".test-quality.toml not found"
fi

# Helper to get TOML values using python
get_toml_value() {
  python3 -c "import tomllib, sys; d=tomllib.load(open('.test-quality.toml', 'rb')); keys=sys.argv[1].split('.'); val=d;
try:
    for k in keys:
        val = val.get(k)
        if val is None: break
    print(val if val is not None else '')
except Exception:
    print('')" "$1"
}

MIN_TESTS=$(get_toml_value "tests.min_test_count")
MIN_RATIO=$(get_toml_value "ratios.min_test_to_source_ratio")

if [ -z "$MIN_TESTS" ]; then
  info "tests.min_test_count not found in .test-quality.toml, skipping test count check"
else
  # 1. Count tests
  # We count workspace-wide test functions in src and crates (if they exist)
  SEARCH_PATHS=""
  [ -d "src" ] && SEARCH_PATHS="$SEARCH_PATHS src/"
  [ -d "crates" ] && SEARCH_PATHS="$SEARCH_PATHS crates/"

  if [ -n "$SEARCH_PATHS" ]; then
    TEST_COUNT=$(rg -c '#\[test\]|#\[tokio::test\]' $SEARCH_PATHS --count-matches 2>/dev/null | awk -F: '{sum += $2} END {print sum}')
    if [ -z "$TEST_COUNT" ]; then TEST_COUNT=0; fi

    if [ "$TEST_COUNT" -lt "$MIN_TESTS" ]; then
      fail "Test count $TEST_COUNT is below minimum $MIN_TESTS"
    fi
    pass "Test count: $TEST_COUNT (min: $MIN_TESTS)"
  else
    info "Neither src/ nor crates/ found, skipping test count check"
  fi
fi

if [ -z "$MIN_RATIO" ]; then
  info "ratios.min_test_to_source_ratio not found in .test-quality.toml, skipping ratio check"
else
  # 2. Test-to-source LOC ratio
  # We aggregate line counts for source files in src/ and crates/
  SEARCH_PATHS=""
  [ -d "src" ] && SEARCH_PATHS="$SEARCH_PATHS src"
  [ -d "crates" ] && SEARCH_PATHS="$SEARCH_PATHS crates"

  if [ -n "$SEARCH_PATHS" ]; then
    TOTAL_RS_LOC=$(find $SEARCH_PATHS -name "*.rs" -exec cat {} + | wc -l)
    # Simplified: count lines in #[cfg(test)] blocks
    # This counts everything after #[cfg(test)] in the file, which usually contains the test module
    TEST_LOC=$(rg -A 10000 "#\[cfg\(test\)\]" $SEARCH_PATHS --no-line-number | grep -v "^--" | wc -l)
    SOURCE_LOC=$((TOTAL_RS_LOC - TEST_LOC))

    if [ "$SOURCE_LOC" -gt 0 ]; then
      # Use python for floating point comparison
      RATIO_OK=$(python3 -c "print(1 if ($TEST_LOC / $SOURCE_LOC) >= $MIN_RATIO else 0)")
      ACTUAL_RATIO=$(python3 -c "print(round($TEST_LOC / $SOURCE_LOC, 2))")

      if [ "$RATIO_OK" -eq 0 ]; then
        fail "Test-to-source ratio $ACTUAL_RATIO is below minimum $MIN_RATIO"
      fi
      pass "Test/Source Ratio: $ACTUAL_RATIO (min: $MIN_RATIO)"
    else
      info "No source LOC found, skipping ratio check"
    fi
  else
    info "Neither src/ nor crates/ found, skipping ratio check"
  fi
fi

# 3. Coverage validation (if cargo-llvm-cov is available)
MIN_COVERAGE=$(get_toml_value "coverage.min_coverage")
if [ -n "$MIN_COVERAGE" ] && command -v cargo-llvm-cov &>/dev/null; then
  info "Checking code coverage (min: ${MIN_COVERAGE}%)..."
  
  # Generate coverage report
  cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info >/dev/null 2>&1 || fail "Coverage generation failed"
  
  # Extract coverage percentage from lcov.info
  if [ -f "lcov.info" ]; then
    # Calculate coverage: (lines hit / total lines) * 100
    LINES_HIT=$(grep -E "^DA:" lcov.info | grep -v ",0$" | wc -l)
    TOTAL_LINES=$(grep -E "^DA:" lcov.info | wc -l)
    
    if [ "$TOTAL_LINES" -gt 0 ]; then
      ACTUAL_COVERAGE=$(python3 -c "print(round(($LINES_HIT / $TOTAL_LINES) * 100, 2))")
      COVERAGE_OK=$(python3 -c "print(1 if $ACTUAL_COVERAGE >= $MIN_COVERAGE else 0)")
      
      if [ "$COVERAGE_OK" -eq 0 ]; then
        fail "Coverage ${ACTUAL_COVERAGE}% is below minimum ${MIN_COVERAGE}%"
      fi
      pass "Coverage: ${ACTUAL_COVERAGE}% (min: ${MIN_COVERAGE}%)"
    else
      info "No coverage data found, skipping coverage check"
    fi
  else
    info "lcov.info not generated, skipping coverage check"
  fi
elif [ -n "$MIN_COVERAGE" ]; then
  info "cargo-llvm-cov not installed, skipping coverage check (run: cargo install cargo-llvm-cov)"
fi

echo ""
echo -e "${GREEN}All quality gates passed!${NC}"
