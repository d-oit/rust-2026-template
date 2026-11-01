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
  # Fallback: read from Cargo.toml (POSIX awk — no gawk 3-arg match)
  MSRV=$(awk '
    /^\[workspace\.package\]/ { in_section=1; next }
    /^\[/                     { in_section=0 }
    in_section && /^rust-version/ {
      if (match($0, /"[^"]+"/)) {
        print substr($0, RSTART + 1, RLENGTH - 2)
        exit
      }
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
# Portable POSIX awk (works with mawk/nawk; avoid gawk-only match 3-arg form)
ROOT_RUST_VERSION=$(awk '
  /^\[workspace\.package\]/ { in_section=1; next }
  /^\[/                     { in_section=0 }
  in_section && /^rust-version/ {
    if (match($0, /"[^"]+"/)) {
      print substr($0, RSTART + 1, RLENGTH - 2)
      exit
    }
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
MEMBERS=$(python3 -c "
import tomllib, glob, os
with open('Cargo.toml', 'rb') as f:
    data = tomllib.load(f)
members = data.get('workspace', {}).get('members', [])
for m in members:
    for expanded in glob.glob(m):
        if os.path.isdir(expanded) and os.path.isfile(os.path.join(expanded, 'Cargo.toml')):
            print(expanded)
" 2>/dev/null || awk '
  /^\[workspace\]/ { in_ws=1; next }
  /^\[/ { in_ws=0; next }
  in_ws && /^members/ {
    if (match($0, /\[.*\]/)) {
      gsub(/[\[\]" ,]/, " ")
      print
      next
    }
    in_members=1
    next
  }
  in_members && /^\]/ { in_members=0; next }
  in_members { gsub(/[[:space:]" ,]/, " "); print }
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
  if grep -q 'version.workspace' "$CARGO_TOML"; then
    # Check if member declares its own rust-version (not workspace inheritance)
    MEMBER_RV=$(grep '^rust-version' "$CARGO_TOML" 2>/dev/null | grep -v 'rust-version.workspace' 2>/dev/null | sed 's/.*"\(.*\)".*/\1/' 2>/dev/null | head -1 || true)
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

# --- Check resolved dependency graph MSRV ---
info "Checking resolved dependency graph MSRV (all features)..."

DEP_AUDIT_STATUS=0
DEP_AUDIT_OUTPUT=$(python3 -c "
import json, subprocess, sys

def parse_version(v_str):
    clean = v_str.strip().lstrip('v')
    parts = clean.split('.')
    if not parts or len(parts) > 3:
        raise ValueError(f'Invalid version: {v_str}')
    major = int(parts[0])
    minor = int(parts[1]) if len(parts) > 1 else 0
    patch = int(parts[2]) if len(parts) > 2 else 0
    return (major, minor, patch)

try:
    ws_v = parse_version('$MSRV')
except Exception as e:
    print(f'ERROR: Invalid workspace MSRV: {e}')
    sys.exit(2)

res = subprocess.run(
    ['cargo', 'metadata', '--format-version', '1', '--all-features'],
    capture_output=True,
    text=True
)

if res.returncode != 0:
    print(f'ERROR: cargo metadata failed: {res.stderr}')
    sys.exit(3)

data = json.loads(res.stdout)
violations = []
for pkg in data.get('packages', []):
    name = pkg.get('name', 'unknown')
    version = pkg.get('version', 'unknown')
    rv = pkg.get('rust_version')
    if rv:
        try:
            pkg_v = parse_version(rv)
            if pkg_v > ws_v:
                violations.append(f\"VIOLATION: Package '{name}' v{version} requires rustc {rv}, which exceeds workspace MSRV $MSRV\")
        except Exception:
            violations.append(f\"VIOLATION: Package '{name}' v{version} declares unparseable rust-version '{rv}'\")

for v in violations:
    print(v)

if violations:
    sys.exit(1)
" 2>&1) || DEP_AUDIT_STATUS=$?

if [[ $DEP_AUDIT_STATUS -eq 0 ]]; then
  pass "All resolved dependencies satisfy workspace MSRV ($MSRV)"
else
  while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    if [[ "$line" =~ ^VIOLATION:\ (.*) ]]; then
      fail "${BASH_REMATCH[1]}"
    else
      fail "$line"
    fi
  done <<< "$DEP_AUDIT_OUTPUT"
fi
echo ""

# --- Verify MSRV toolchain compiles workspace ---
info "Verifying MSRV toolchain compiles workspace (all targets, all features)..."

CARGO_CMD=()
CURRENT_RUSTC=$(rustc --version 2>/dev/null || echo "")

if [[ "$CURRENT_RUSTC" == *"$MSRV"* ]]; then
  CARGO_CMD=(cargo check --workspace --all-targets --all-features)
elif command -v rustup &>/dev/null && rustup toolchain list 2>/dev/null | grep -q "$MSRV"; then
  CARGO_CMD=(cargo +"$MSRV" check --workspace --all-targets --all-features)
fi

if [[ ${#CARGO_CMD[@]} -gt 0 ]]; then
  COMPILE_OUTPUT=$("${CARGO_CMD[@]}" 2>&1) && COMPILE_STATUS=0 || COMPILE_STATUS=$?
  if [[ $COMPILE_STATUS -eq 0 ]]; then
    pass "Workspace compiles with MSRV toolchain $MSRV (--all-targets --all-features)"
  else
    fail "Workspace does not compile with MSRV toolchain $MSRV:"
    echo "$COMPILE_OUTPUT"
  fi
else
  warn "MSRV toolchain $MSRV not active or installed via rustup, skipping compilation check"
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
