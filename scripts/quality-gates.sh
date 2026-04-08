#!/usr/bin/env bash
# scripts/quality-gates.sh
# Run all quality gates locally - mirrors CI pipeline
# Usage: ./scripts/quality-gates.sh [--fix]

set -euo pipefail

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
info "[1/8] Checking formatting..."
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
info "[2/8] Running Clippy..."
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
info "[3/8] Building..."
cargo build --all-targets || fail "Build: failed"
pass "Build: OK"

# ============================================================
# 4. TESTS
# ============================================================
info "[4/8] Running tests..."
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
info "[5/8] Security audit..."
if command -v cargo-audit &>/dev/null; then
  cargo audit || fail "Security audit: vulnerabilities found"
  pass "Audit: OK"
else
  info "cargo-audit not installed, skipping (run: cargo install cargo-audit)"
fi

# ============================================================
# 6. SUPPLY CHAIN (cargo-deny)
# ============================================================
info "[6/8] Supply chain check..."
if command -v cargo-deny &>/dev/null; then
  cargo deny check || fail "cargo-deny: violations found"
  pass "Deny: OK"
else
  info "cargo-deny not installed, skipping (run: cargo install cargo-deny)"
fi

# ============================================================
# 7. UNUSED DEPENDENCIES
# ============================================================
info "[7/8] Checking unused dependencies..."
if command -v cargo-machete &>/dev/null; then
  cargo machete || fail "Unused deps found"
  pass "Machete: OK"
else
  info "cargo-machete not installed, skipping (run: cargo install cargo-machete)"
fi

# ============================================================
# 8. PRIVACY CHECK (No emails)
# ============================================================
info "[8/8] Checking for email addresses (privacy-first)..."
EMAIL_PATTERN='[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}'
EXCLUDE_PATTERN='example\.com|example\.org|test\.com|\.git|target|\.agents'

# ⚡ Bolt: Optimized by using --exclude-dir to skip large/irrelevant directories
# instead of filtering results after a full recursive scan. This significantly
# reduces I/O and CPU time in large Rust projects with deep target/ folders.
if grep -rE "$EMAIL_PATTERN" \
  --exclude-dir=.git --exclude-dir=target --exclude-dir=.agents \
  . 2>/dev/null | grep -vE "$EXCLUDE_PATTERN"; then
  fail "Email address detected in codebase. Please remove it to comply with privacy-first policy."
else
  pass "Privacy: OK"
fi

echo ""
echo -e "${GREEN}All quality gates passed!${NC}"
