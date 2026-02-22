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
info "[1/7] Checking formatting..."
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
info "[2/7] Running Clippy..."
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
info "[3/7] Building..."
cargo build --all-targets || fail "Build: failed"
pass "Build: OK"

# ============================================================
# 4. TESTS
# ============================================================
info "[4/7] Running tests..."
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
info "[5/7] Security audit..."
if command -v cargo-audit &>/dev/null; then
  cargo audit || fail "Security audit: vulnerabilities found"
  pass "Audit: OK"
else
  info "cargo-audit not installed, skipping (run: cargo install cargo-audit)"
fi

# ============================================================
# 6. SUPPLY CHAIN (cargo-deny)
# ============================================================
info "[6/7] Supply chain check..."
if command -v cargo-deny &>/dev/null; then
  cargo deny check || fail "cargo-deny: violations found"
  pass "Deny: OK"
else
  info "cargo-deny not installed, skipping (run: cargo install cargo-deny)"
fi

# ============================================================
# 7. UNUSED DEPENDENCIES
# ============================================================
info "[7/7] Checking unused dependencies..."
if command -v cargo-machete &>/dev/null; then
  cargo machete || fail "Unused deps found"
  pass "Machete: OK"
else
  info "cargo-machete not installed, skipping (run: cargo install cargo-machete)"
fi

echo ""
echo -e "${GREEN}All quality gates passed!${NC}"
