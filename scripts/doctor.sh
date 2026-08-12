#!/usr/bin/env bash
# doctor.sh - Environment diagnostics for this Rust template.
# Checks required/optional tools, git state, symlinks, hooks, and core files.
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT" || exit 1

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

ISSUES=0

pass() { printf "  ${GREEN}✓${NC} %s\n" "$1"; }
fail() { printf "  ${RED}✗${NC} %s\n" "$1"; ISSUES=$((ISSUES + 1)); }
warn() { printf "  ${YELLOW}!${NC} %s\n" "$1"; }

echo "=== Environment Diagnostics ==="
echo ""

# --- Required tools ---
echo "Required tools:"
for cmd in git cargo; do
  if command -v "$cmd" &>/dev/null; then
    pass "$cmd: $(command -v "$cmd")"
  else
    fail "$cmd: not found"
  fi
done
echo ""

# --- Optional quality tools ---
echo "Optional quality tools:"
for cmd in cargo-nextest cargo-audit cargo-deny cargo-machete shellcheck markdownlint-cli2; do
  if command -v "$cmd" &>/dev/null; then
    pass "$cmd: installed"
  else
    warn "$cmd: not installed (run: cargo install $cmd or npm i -g $cmd)"
  fi
done
echo ""

# --- Optional workflow tools ---
echo "Optional workflow tools:"
if command -v gh &>/dev/null; then
  pass "gh CLI: $(command -v gh)"
  if gh extension list 2>/dev/null | grep -qw gh-stack; then
    pass "gh-stack: installed"
  else
    warn "gh-stack: not installed (optional, for stacked PRs: gh extension install github/gh-stack)"
  fi
else
  warn "gh CLI: not installed (optional, needed for PR workflows and stacked PRs)"
fi
echo ""

# --- Linker check ---
echo "Linker configuration:"
bash "$(dirname "${BASH_SOURCE[0]}")/check-linker.sh" 2>/dev/null || warn "check-linker.sh not found or failed"
echo ""

# --- Git state ---
echo "Git state:"
BRANCH=$(git branch --show-current 2>/dev/null || echo "detached")
pass "Current branch: $BRANCH"

if git diff --quiet 2>/dev/null && git diff --cached --quiet 2>/dev/null; then
  pass "Working tree clean"
else
  warn "Working tree has uncommitted changes"
fi
echo ""

# --- Symlinks ---
echo "Skill symlinks:"
for cli_dir in .claude/skills .qwen/skills; do
  if [[ -d "$cli_dir" ]]; then
    COUNT=$(find "$cli_dir" -type l 2>/dev/null | wc -l)
    if [[ "$COUNT" -gt 0 ]]; then
      pass "$cli_dir: $COUNT symlinks"
    else
      warn "$cli_dir: no symlinks (run ./scripts/setup-skills.sh)"
    fi
  else
    warn "$cli_dir: directory missing"
  fi
done
echo ""

# --- Git hooks ---
echo "Git hooks:"
HOOKS_PATH=$(git config core.hooksPath 2>/dev/null || echo "")
if [[ "$HOOKS_PATH" == ".githooks" ]]; then
  if [[ -f ".githooks/pre-commit" ]]; then
    pass "pre-commit hook installed"
  else
    warn "core.hooksPath set but .githooks/pre-commit missing"
  fi
else
  warn "core.hooksPath not set to .githooks (run: git config core.hooksPath .githooks)"
fi
echo ""

# --- Core files ---
echo "Core files:"
for file in AGENTS.md CHANGELOG.md Cargo.toml rust-toolchain.toml deny.toml; do
  if [[ -f "$file" ]]; then
    pass "$file"
  else
    fail "$file: missing"
  fi
done
echo ""

# --- Skills ---
echo "Skills:"
if [[ -d ".agents/skills" ]]; then
  SKILL_COUNT=$(find .agents/skills -name "SKILL.md" -type f 2>/dev/null | wc -l)
  pass ".agents/skills: $SKILL_COUNT skills"
else
  fail ".agents/skills: directory missing"
fi
echo ""

# --- Summary ---
if [[ $ISSUES -eq 0 ]]; then
  echo -e "${GREEN}All checks passed.${NC}"
else
  echo -e "${RED}$ISSUES issue(s) found.${NC}"
fi
