#!/usr/bin/env bash
# scripts/propagate-version.sh
# Reads VERSION file and propagates to workspace.package.version in root Cargo.toml.
# Validates consistency across all workspace members.
# Usage: ./scripts/propagate-version.sh [--check]
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT" || exit 1

# --- Colors ---
if [[ -t 1 ]] && [[ "${FORCE_COLOR:-}" != "0" ]]; then
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  YELLOW='\033[1;33m'
  NC='\033[0m'
else
  RED=''
  GREEN=''
  YELLOW=''
  NC=''
fi

# --- Parse arguments ---
CHECK_ONLY=false
for arg in "$@"; do
  case $arg in
    --check) CHECK_ONLY=true ;;
    *) echo "Unknown argument: $arg"; exit 1 ;;
  esac
done

# --- Read VERSION file ---
if [[ ! -f "VERSION" ]]; then
  echo -e "${RED}[ERROR]${NC} VERSION file not found"
  exit 1
fi

VERSION=$(cat VERSION | tr -d '[:space:]')
if [[ -z "$VERSION" ]]; then
  echo -e "${RED}[ERROR]${NC} VERSION file is empty"
  exit 1
fi

echo "VERSION file: $VERSION"

# --- Validate version format ---
if ! echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$'; then
  echo -e "${RED}[ERROR]${NC} Invalid version format: $VERSION (expected X.Y.Z or X.Y.Z-qualifier)"
  exit 1
fi

# --- Check root Cargo.toml workspace.package.version ---
if [[ ! -f "Cargo.toml" ]]; then
  echo -e "${RED}[ERROR]${NC} Root Cargo.toml not found"
  exit 1
fi

ROOT_VERSION=$(python3 -c "
import tomllib
with open('Cargo.toml', 'rb') as f:
    data = tomllib.load(f)
print(data.get('workspace', {}).get('package', {}).get('version', 'NOT_FOUND'))
" 2>/dev/null || echo "PARSE_ERROR")

if [[ "$ROOT_VERSION" == "PARSE_ERROR" ]]; then
  # Fallback to awk if python3 tomllib unavailable
  ROOT_VERSION=$(awk '
    /^\[workspace\.package\]/ { in_section=1; next }
    /^\[/                     { in_section=0 }
    in_section && /^version[[:space:]]*=/ {
      match($0, /"([^"]+)"/, arr)
      if (arr[1] != "") { print arr[1]; exit }
    }
  ' Cargo.toml)
fi

if [[ -z "$ROOT_VERSION" || "$ROOT_VERSION" == "NOT_FOUND" ]]; then
  echo -e "${RED}[ERROR]${NC} Could not read workspace.package.version from Cargo.toml"
  exit 1
fi

echo "Cargo.toml workspace version: $ROOT_VERSION"

# --- Compare ---
if [[ "$VERSION" != "$ROOT_VERSION" ]]; then
  if $CHECK_ONLY; then
    echo -e "${RED}[FAIL]${NC} Version mismatch: VERSION=$VERSION, Cargo.toml=$ROOT_VERSION"
    exit 1
  else
    echo -e "${YELLOW}[UPDATE]${NC} Updating Cargo.toml workspace version: $ROOT_VERSION -> $VERSION"
    python3 -c "
import re
with open('Cargo.toml', 'r') as f:
    content = f.read()

in_section = False
lines = content.split('\n')
for i, line in enumerate(lines):
    if re.match(r'^\[workspace\.package\]', line):
        in_section = True
        continue
    if re.match(r'^\[', line):
        in_section = False
        continue
    if in_section and re.match(r'^version\s*=', line):
        lines[i] = f'version = \"{VERSION}\"'
        break

with open('Cargo.toml', 'w') as f:
    f.write('\n'.join(lines))
" 2>/dev/null || {
      # Fallback: sed-based replacement
      sed -i "/^\[workspace\.package\]/,/^\[/ s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
    }
    echo -e "${GREEN}[OK]${NC} Cargo.toml updated to $VERSION"
  fi
else
  echo -e "${GREEN}[OK]${NC} Versions match: $VERSION"
fi

# --- Check member crates inherit workspace version ---
echo ""
echo "Checking workspace member version inheritance..."
MEMBER_COUNT=0
INHERIT_COUNT=0

for member_dir in $(cargo metadata --format-version 1 --no-deps 2>/dev/null | python3 -c "
import json, sys
meta = json.load(sys.stdin)
for p in meta.get('packages', []):
    if p.get('source') is None:  # workspace members only
        import os
        print(os.path.relpath(p['manifest_path'], '.'))
" 2>/dev/null); do
  MEMBER_COUNT=$((MEMBER_COUNT + 1))
  if grep -q 'version.workspace\s*=\s*true' "$member_dir" 2>/dev/null; then
    INHERIT_COUNT=$((INHERIT_COUNT + 1))
  fi
done

if [[ $MEMBER_COUNT -gt 0 ]]; then
  echo "Members inheriting workspace version: $INHERIT_COUNT / $MEMBER_COUNT"
fi

echo ""
echo -e "${GREEN}Version propagation complete${NC}"
