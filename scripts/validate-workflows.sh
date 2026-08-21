#!/usr/bin/env bash
# scripts/validate-workflows.sh
# Validates GitHub Actions workflows for syntax, SHA pinning, and best practices.
# Usage: ./scripts/validate-workflows.sh
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
WORKFLOWS_DIR=".github/workflows"

if [[ ! -d "$WORKFLOWS_DIR" ]]; then
  echo -e "${RED}[ERROR]${NC} Workflows directory not found: $WORKFLOWS_DIR"
  exit 1
fi

info "Validating GitHub Actions workflows..."
echo ""

# 1. YAML syntax check (optional — skip cleanly when PyYAML/yq missing)
info "Checking YAML syntax..."
YAML_OK=true
YAML_SKIPPED=false
if command -v python3 &>/dev/null && python3 -c "import yaml" 2>/dev/null; then
  for f in "$WORKFLOWS_DIR"/*.yml "$WORKFLOWS_DIR"/*.yaml; do
    [[ -f "$f" ]] || continue
    if ! python3 -c "import yaml; yaml.safe_load(open('$f'))" 2>/dev/null; then
      fail "YAML syntax error: $f"
      YAML_OK=false
    fi
  done
elif command -v yq &>/dev/null; then
  for f in "$WORKFLOWS_DIR"/*.yml "$WORKFLOWS_DIR"/*.yaml; do
    [[ -f "$f" ]] || continue
    if ! yq '.' "$f" >/dev/null 2>&1; then
      fail "YAML syntax error: $f"
      YAML_OK=false
    fi
  done
else
  warn "PyYAML/yq not installed — skipping YAML syntax check (optional: pip install pyyaml)"
  YAML_SKIPPED=true
fi
if $YAML_OK && ! $YAML_SKIPPED; then
  pass "YAML syntax: all valid"
elif $YAML_OK && $YAML_SKIPPED; then
  pass "YAML syntax: skipped (no parser)"
fi
echo ""

# 2. SHA pinning check
info "Checking action SHA pinning..."
PINNING_ISSUES=0
for f in "$WORKFLOWS_DIR"/*.yml "$WORKFLOWS_DIR"/*.yaml; do
  [[ -f "$f" ]] || continue
  # Find uses: lines that reference actions with tags instead of SHAs
  while IFS= read -r line; do
    # Skip local actions (./) and reusable workflows
    if echo "$line" | grep -qE 'uses:\s*\./'; then
      continue
    fi
    # Check for tag-based refs (v1, v2, main, master) without SHA
    if echo "$line" | grep -qE 'uses:\s*[a-zA-Z0-9_-]+/[a-zA-Z0-9_-]+@[^#]+[^@]*$'; then
      ref=$(echo "$line" | sed -n 's/.*uses:\s*.*@\([^#]*\).*/\1/p' | tr -d ' ')
      # Allow SHA refs (40+ hex chars) and local paths
      if [[ ${#ref} -lt 40 ]] && ! echo "$ref" | grep -qE '^[a-f0-9]{40,}$'; then
        warn "Tag-based ref in $(basename "$f"): $ref (prefer SHA pinning)"
        PINNING_ISSUES=$((PINNING_ISSUES + 1))
      fi
    fi
  done < <(grep -n 'uses:' "$f" 2>/dev/null || true)
done
if [[ $PINNING_ISSUES -eq 0 ]]; then
  pass "SHA pinning: all actions pinned"
else
  warn "SHA pinning: $PINNING_ISSUES tag-based references found"
fi
echo ""

# 3. Required fields check
info "Checking workflow structure..."
STRUCTURE_OK=true
for f in "$WORKFLOWS_DIR"/*.yml "$WORKFLOWS_DIR"/*.yaml; do
  [[ -f "$f" ]] || continue
  fname=$(basename "$f")
  if ! grep -q '^name:' "$f"; then
    warn "$fname: missing 'name' field"
    STRUCTURE_OK=false
  fi
  if ! grep -q '^on:' "$f" && ! grep -q '^"on":' "$f"; then
    warn "$fname: missing 'on' trigger"
    STRUCTURE_OK=false
  fi
  if ! grep -q '^jobs:' "$f"; then
    warn "$fname: missing 'jobs' section"
    STRUCTURE_OK=false
  fi
done
if $STRUCTURE_OK; then
  pass "Workflow structure: all valid"
fi
echo ""

# 4. Permissions check
info "Checking permissions declarations..."
PERM_ISSUES=0
for f in "$WORKFLOWS_DIR"/*.yml "$WORKFLOWS_DIR"/*.yaml; do
  [[ -f "$f" ]] || continue
  fname=$(basename "$f")
  # Check if top-level permissions are declared (restrictive by default)
  if ! grep -q '^permissions:' "$f"; then
    warn "$fname: no top-level permissions (defaults to broad permissions)"
    PERM_ISSUES=$((PERM_ISSUES + 1))
  fi
done
if [[ $PERM_ISSUES -eq 0 ]]; then
  pass "Permissions: all workflows declare permissions"
else
  warn "Permissions: $PERM_ISSUES workflows without explicit permissions"
fi
echo ""

# Summary
if [[ $FAILED -ne 0 ]]; then
  echo -e "${RED}Workflow validation FAILED${NC}"
  exit 1
fi

echo -e "${GREEN}Workflow validation passed${NC}"
