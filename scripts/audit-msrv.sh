#!/usr/bin/env bash
# scripts/audit-msrv.sh
# Validates MSRV compliance across all workspace members.
# Usage: ./scripts/audit-msrv.sh [--fix]
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT" || exit 1

# --- Colors ---
if [[ -t 1 ]] && [[ "${FORCE_COLOR:-}" != "0" ]]; then
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  YELLOW='\033[1;33m'
  BLUE='\033[0;34m'
  NC='\033[0m'
else
  RED=''
  GREEN=''
  YELLOW=''
  BLUE=''
  NC=''
fi

pass() { echo -e "${GREEN}[PASS]${NC} $1"; }
fail() { echo -e "${RED}[FAIL]${NC} $1"; FAILED=1; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
info() { echo -e "${BLUE}[INFO]${NC} $1"; }

FAILED=0

# --- Read workspace MSRV from rust-toolchain.toml ---
if [[ -f "rust-toolchain.toml" ]]; then
  TOOLCHAIN=$(grep '^channel' rust-toolchain.toml | sed 's/.*"\(.*\)".*/\1/')
  # Extract just the version number
  MSRV=$(echo "$TOOLCHAIN" | grep -oE '[0-9]+\.[0-9]+' | head -1)
else
  # Fallback: read from Cargo.toml
  MSRV=$(awk '
    /^\[workspace\.package\]/ { in_section=1; next }
    /^\[/                     { in_section=0 }
    in_section && /^rust-version/ {
      match($0, /"([^"]+)"/, arr)
      if (arr[1] != "") { print arr[1]; exit }
    }
  ' Cargo.toml)
fi

if [[ -z "$MSRV" ]]; then
  echo -e "${RED}[ERROR]${NC} Could not determine MSRV from rust-toolchain.toml or Cargo.toml"
  exit 1
fi

info "Workspace MSRV: $MSRV"
echo ""

# --- Check root Cargo.toml rust-version ---
info "Checking root Cargo.toml rust-version field..."
ROOT_RUST_VERSION=$(awk '
  /^\[workspace\.package\]/ { in_section=1; next }
  /^\[/                     { in_section=0 }
  in_section && /^rust-version/ {
    match($0, /"([^"]+)"/, arr)
    if (arr[1] != "") { print arr[1]; exit }
  }
' Cargo.toml)

if [[ -n "$ROOT_RUST_VERSION" ]]; then
  if [[ "$ROOT_RUST_VERSION" == "$MSRV" ]]; then
    pass "Root rust-version matches MSRV: $ROOT_RUST_VERSION"
  else
    warn "Root rust-version ($ROOT_RUST_VERSION) differs from MSRV ($MSRV)"
  fi
else
  warn "No rust-version field in workspace.package"
fi
echo ""

# --- Check workspace member crates ---
info "Checking workspace member crates..."

# Get workspace members from Cargo.toml
MEMBERS=$(awk '
  /^\[workspace\]/ { in_ws=1; next }
  /^\[/ { in_ws=0; next }
  in_ws && /^members/ {
    # Handle inline array: members = ["a", "b"]
    if (match($0, /\[.*\]/)) {
      gsub(/[\[\]"',]/, " ")
      print
      next
    }
    # Handle multi-line array
    in_members=1
    next
  }
  in_members && /^\]/ { in_members=0; next }
  in_members { gsub(/[[:space:]]*["',]/, " "); print }
' Cargo.toml)

ISSUES=0
for member in $MEMBERS; do
  member=$(echo "$member" | tr -d '[:space:]')
  [[ -z "$member" ]] && continue

  CARGO_TOML="$member/Cargo.toml"
  if [[ ! -f "$CARGO_TOML" ]]; then
    warn "Member crate not found: $CARGO_TOML"
    continue
  fi

  # Check if member inherits workspace version
  if grep -q 'version.workspace\s*=\s*true' "$CARGO_TOML"; then
    # Check if member declares its own rust-version
    MEMBER_RV=$(grep 'rust-version' "$CARGO_TOML" | sed 's/.*"\(.*\)".*/\1/' | head -1)
    if [[ -n "$MEMBER_RV" ]]; then
      if [[ "$MEMBER_RV" != "$MSRV" ]]; then
        warn "$member: rust-version ($MEMBER_RV) differs from MSRV ($MSRV)"
        ISSUES=$((ISSUES + 1))
      fi
    fi
    pass "$member: inherits workspace version"
  else
    warn "$member: does not inherit workspace version"
    ISSUES=$((ISSUES + 1))
  fi
done
echo ""

# --- Verify MSRV toolchain compiles workspace ---
info "Verifying MSRV toolchain compiles workspace..."

# Check if the MSRV toolchain is installed
if command -v rustup &>/dev/null; then
  if rustup toolchain list | grep -q "$MSRV"; then
    if ! cargo +"$MSRV" check --workspace 2>/dev/null; then
      fail "Workspace does not compile with MSRV toolchain $MSRV"
    else
      pass "Workspace compiles with MSRV toolchain $MSRV"
    fi
  else
    warn "MSRV toolchain $MSRV not installed, skipping compilation check"
  fi
else
  warn "rustup not found, skipping MSRV compilation check"
fi
echo ""

# --- Summary ---
if [[ $FAILED -ne 0 ]]; then
  echo -e "${RED}MSRV audit FAILED${NC}"
  exit 1
fi

if [[ $ISSUES -gt 0 ]]; then
  echo -e "${YELLOW}MSRV audit passed with $ISSUES warnings${NC}"
else
  echo -e "${GREEN}MSRV audit passed${NC}"
fi
