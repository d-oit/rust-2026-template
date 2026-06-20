#!/usr/bin/env bash
# scripts/check-crate-publish.sh
# Validates that all workspace crates are publishable to crates.io.
# Usage: ./scripts/check-crate-publish.sh
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

info "Checking crate publish readiness..."
echo ""

# Get workspace members that are publishable
MEMBERS=$(python3 -c "
import tomllib, glob
with open('Cargo.toml', 'rb') as f:
    data = tomllib.load(f)
members = data.get('workspace', {}).get('members', [])
for m in members:
    for expanded in glob.glob(m):
        print(expanded)
" 2>/dev/null || echo "")

for member in $MEMBERS; do
  member=$(echo "$member" | tr -d '[:space:]')
  [[ -z "$member" ]] && continue

  CARGO_TOML="$member/Cargo.toml"
  if [[ ! -f "$CARGO_TOML" ]]; then
    warn "Member crate not found: $CARGO_TOML"
    continue
  fi

  # Skip crates marked publish = false
  if grep -q 'publish\s*=\s*false' "$CARGO_TOML"; then
    info "$member: skipped (publish = false)"
    continue
  fi

  # Check required metadata
  CRATE_NAME=$(grep '^name' "$CARGO_TOML" | head -1 | sed 's/.*"\(.*\)".*/\1/' | tr -d '[:space:]')
  if [[ -z "$CRATE_NAME" ]]; then
    fail "$member: missing crate name"
    continue
  fi

  # Check description
  if ! grep -q 'description' "$CARGO_TOML"; then
    warn "$member: missing description"
  fi

  # Check license
  if ! grep -qE '(license|license-file)' "$CARGO_TOML"; then
    warn "$member: missing license"
  fi

  # Check README exists
  if [[ ! -f "$member/README.md" ]] && [[ ! -f "$member/readme.md" ]]; then
    # Check if README is specified in Cargo.toml
    if ! grep -q 'readme' "$CARGO_TOML"; then
      warn "$member: no README.md found"
    fi
  fi

  # Dry-run package (faster than full publish)
  if ! OUTPUT=$(cargo package --list -p "$CRATE_NAME" 2>&1); then
    fail "$member: package listing failed"
    echo "$OUTPUT" | head -10 | sed 's/^/    /'
  else
    pass "$member: packaging OK"
  fi
done

echo ""

if [[ $FAILED -ne 0 ]]; then
  echo -e "${RED}Crate publish check FAILED${NC}"
  exit 1
fi

echo -e "${GREEN}Crate publish check passed${NC}"
