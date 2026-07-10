#!/usr/bin/env bash
# scripts/quality-gates.sh
# Full quality gate with auto-detection for multiple languages.
# Usage: ./scripts/quality-gates.sh [--fix]
# Exit 0 = success, Exit 1 = errors.
set +e
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT" || exit 1

# --- Configuration ---
readonly GIT_EXCLUDE="./.git/*"
readonly MAX_LINES_PER_SOURCE_FILE=500

# --- Parse arguments ---
FIX=false
for arg in "$@"; do
  case $arg in
    --fix) FIX=true ;;
    *) echo "Unknown argument: $arg"; exit 1 ;;
  esac
done

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

# Ensure a tool is available, installing it if missing.
# Usage: ensure_tool <command> <install_method>
#   install_method: "cargo:<crate>" | "apt:<pkg>" | "brew:<pkg>" | "npm:<pkg>"
# Returns 0 if tool is available, 1 if not.
ensure_tool() {
  local cmd="$1"
  local install_method="${2:-}"

  if command -v "$cmd" &>/dev/null; then
    return 0
  fi

  warn "$cmd not found — attempting install..."
  case "$install_method" in
    cargo:*)
      local crate="${install_method#cargo:}"
      if cargo install "$crate" 2>&1 | tail -1; then
        info "$cmd installed via cargo install $crate"
        return 0
      fi
      ;;
    apt:*)
      local pkg="${install_method#apt:}"
      if sudo apt-get update -qq && sudo apt-get install -y -qq "$pkg" 2>&1 | tail -1; then
        info "$cmd installed via apt install $pkg"
        return 0
      fi
      ;;
    brew:*)
      local pkg="${install_method#brew:}"
      if brew install "$pkg" 2>&1 | tail -1; then
        info "$cmd installed via brew install $pkg"
        return 0
      fi
      ;;
    npm:*)
      local pkg="${install_method#npm:}"
      if npm install -g "$pkg" 2>&1 | tail -1; then
        info "$cmd installed via npm install -g $pkg"
        return 0
      fi
      ;;
    none)
      return 1
      ;;
  esac

  warn "$cmd install failed — skipping"
  return 1
}

FAILED=0
DETECTED_LANGUAGES=()

# Determine context
GITHUB_EVENT="${GITHUB_EVENT:-${GITHUB_EVENT_NAME:-}}"
GITHUB_REF="${GITHUB_REF:-}"

printf "Running quality gate...\n\n"

# ============================================================
# 1. LANGUAGE DETECTION
# ============================================================
info "Detecting project languages..."

if [[ -f "Cargo.toml" ]]; then
  printf "  ${GREEN}✓${NC} Rust (Cargo.toml)\n"
  DETECTED_LANGUAGES+=("rust")
fi

if [[ -f "package.json" ]]; then
  printf "  ${GREEN}✓${NC} TypeScript/JavaScript (package.json)\n"
  DETECTED_LANGUAGES+=("typescript")
fi

if [[ -f "requirements.txt" ]] || [[ -f "pyproject.toml" ]] || [[ -f "setup.py" ]]; then
  printf "  ${GREEN}✓${NC} Python (requirements.txt/pyproject.toml)\n"
  DETECTED_LANGUAGES+=("python")
fi

if [[ -f "go.mod" ]]; then
  printf "  ${GREEN}✓${NC} Go (go.mod)\n"
  DETECTED_LANGUAGES+=("go")
fi

if find . -name "*.sh" -not -path "$GIT_EXCLUDE" -not -path "./target/*" -print -quit 2>/dev/null | grep -q .; then
  printf "  ${GREEN}✓${NC} Shell scripts detected\n"
  DETECTED_LANGUAGES+=("shell")
fi

if find . -name "*.md" -not -path "$GIT_EXCLUDE" -not -path "./target/*" -print -quit 2>/dev/null | grep -q .; then
  printf "  ${GREEN}✓${NC} Markdown files detected\n"
  DETECTED_LANGUAGES+=("markdown")
fi

if [[ ${#DETECTED_LANGUAGES[@]} -eq 0 ]]; then
  warn "No recognized project files found."
fi
printf "\n"

# ============================================================
# 2. LOC LIMITS
# ============================================================
info "Enforcing LOC limits (max ${MAX_LINES_PER_SOURCE_FILE} lines per file)..."
LOC_VIOLATIONS=0
while IFS= read -r file; do
  lines=$(wc -l < "$file" 2>/dev/null || echo 0)
  if [[ "$lines" -gt "$MAX_LINES_PER_SOURCE_FILE" ]]; then
    warn "  $file: $lines lines (max $MAX_LINES_PER_SOURCE_FILE)"
    LOC_VIOLATIONS=$((LOC_VIOLATIONS + 1))
  fi
done < <(find . -name "*.rs" -not -path "./target/*" -not -path "./.git/*" -type f 2>/dev/null)

if [[ $LOC_VIOLATIONS -gt 0 ]]; then
  fail "LOC: $LOC_VIOLATIONS files exceed ${MAX_LINES_PER_SOURCE_FILE} lines"
else
  pass "LOC: All source files within limit"
fi
printf "\n"

# ============================================================
# 3. SKILL VALIDATION
# ============================================================
info "Validating skills..."
if [[ -f "./scripts/validate-skills.sh" ]]; then
  if ./scripts/validate-skills.sh >/dev/null 2>&1; then
    pass "Skills: valid"
  else
    warn "Skills: validation reported issues (run ./scripts/validate-skills.sh for details)"
  fi
else
  warn "Skills: validate-skills.sh not found"
fi
printf "\n"

# ============================================================
# 4. ADR COMPLIANCE
# ============================================================
if [[ -f "./scripts/check-adr-compliance.sh" ]]; then
  info "Checking ADR compliance..."
  if ./scripts/check-adr-compliance.sh >/dev/null 2>&1; then
    pass "ADR: compliant"
  else
    warn "ADR: compliance check failed (run ./scripts/check-adr-compliance.sh for details)"
  fi
  printf "\n"
fi

# ============================================================
# 5. RUST CHECKS
# ============================================================
if [[ " ${DETECTED_LANGUAGES[*]} " =~ " rust " ]]; then
  info "Running Rust checks..."

  # Format
  if $FIX; then
    cargo fmt --all
    pass "Format: auto-fixed"
  else
    if ! OUTPUT=$(cargo fmt --all -- --check 2>&1); then
      fail "Format: run 'cargo fmt --all' to fix"
      printf "%s\n" "$OUTPUT" >&2
    else
      pass "Format: OK"
    fi
  fi

  # Clippy
  if $FIX; then
    cargo clippy --fix --allow-dirty --allow-staged --all-targets --all-features
    pass "Clippy: auto-fixed"
  else
    if ! OUTPUT=$(cargo clippy --all-targets --all-features -- -D warnings 2>&1); then
      fail "Clippy: fix lint errors above"
      printf "%s\n" "$OUTPUT" >&2
    else
      pass "Clippy: OK"
    fi
  fi

  # Build
  if ! OUTPUT=$(cargo build --all-targets 2>&1); then
    fail "Build: failed"
    printf "%s\n" "$OUTPUT" >&2
  else
    pass "Build: OK"
  fi

  # Tests
  if ensure_tool cargo-nextest "cargo:cargo-nextest"; then
    if ! OUTPUT=$(cargo nextest run --all-features --workspace 2>&1); then
      fail "Tests: failed"
      printf "%s\n" "$OUTPUT" >&2
    else
      pass "Tests (nextest): OK"
    fi
  else
    if ! OUTPUT=$(cargo test --all-features --workspace 2>&1); then
      fail "Tests: failed"
      printf "%s\n" "$OUTPUT" >&2
    else
      pass "Tests: OK"
    fi
  fi

  # Doc tests
  if ! OUTPUT=$(cargo test --doc --all-features 2>&1); then
    fail "Doc tests: failed"
    printf "%s\n" "$OUTPUT" >&2
  else
    pass "Doc tests: OK"
  fi

  # Security audit
  if ensure_tool cargo-audit "cargo:cargo-audit"; then
    AUDIT_OUTPUT=$(cargo audit 2>&1) && AUDIT_EXIT=$? || AUDIT_EXIT=$?
    if [ $AUDIT_EXIT -ne 0 ]; then
      if echo "$AUDIT_OUTPUT" | grep -q "unsupported CVSS version"; then
        warn "cargo-audit: Skipping due to RustSec advisory format issue"
      else
        fail "Security audit: vulnerabilities found"
      fi
    else
      pass "Audit: OK"
    fi
  else
    warn "cargo-audit: unavailable, skipping"
  fi

  # Supply chain
  if ensure_tool cargo-deny "cargo:cargo-deny"; then
    if ! OUTPUT=$(cargo deny check 2>&1); then
      fail "cargo-deny: violations found"
      printf "%s\n" "$OUTPUT" >&2
    else
      pass "Deny: OK"
    fi
  else
    warn "cargo-deny: unavailable, skipping"
  fi

  # Unused dependencies
  if ensure_tool cargo-machete "cargo:cargo-machete"; then
    if ! OUTPUT=$(cargo machete 2>&1); then
      fail "Unused deps found"
      printf "%s\n" "$OUTPUT" >&2
    else
      pass "Machete: OK"
    fi
  else
    warn "cargo-machete: unavailable, skipping"
  fi

  # MSRV audit
  if [[ -f "./scripts/audit-msrv.sh" ]]; then
    if ./scripts/audit-msrv.sh >/dev/null 2>&1; then
      pass "MSRV: compliant"
    else
      warn "MSRV: audit found issues (run ./scripts/audit-msrv.sh for details)"
    fi
  fi

  printf "\n"
fi

# ============================================================
# 6. SHELL CHECKS
# ============================================================
if [[ " ${DETECTED_LANGUAGES[*]} " =~ " shell " ]]; then
  info "Running Shell script checks..."
  if ensure_tool shellcheck "apt:shellcheck"; then
    TMP_SH_LIST=$(mktemp)
    find . -name "*.sh" -not -path "$GIT_EXCLUDE" -not -path "./target/*" -print0 2>/dev/null > "$TMP_SH_LIST" || true
    if [[ -s "$TMP_SH_LIST" ]]; then
      if ! xargs -0 shellcheck --severity=error < "$TMP_SH_LIST" 2>/dev/null; then
        fail "shellcheck failed"
      else
        pass "shellcheck: OK"
      fi
    fi
    rm -f -- "$TMP_SH_LIST"
  else
    warn "shellcheck: unavailable, skipping"
  fi
  printf "\n"
fi

# ============================================================
# 7. MARKDOWN CHECKS
# ============================================================
if [[ " ${DETECTED_LANGUAGES[*]} " =~ " markdown " ]]; then
  info "Running Markdown checks..."
  if ensure_tool markdownlint-cli2 "npm:markdownlint-cli2"; then
    if ! markdownlint-cli2 "**/*.md" >/dev/null 2>&1; then
      warn "markdownlint-cli2: issues found (run 'markdownlint-cli2 \"**/*.md\"' locally)"
    else
      pass "markdownlint: OK"
    fi
  else
    warn "markdownlint-cli2: unavailable, skipping"
  fi
  printf "\n"
fi

# ============================================================
# 8. PRIVACY CHECK (No emails)
# ============================================================
info "Checking for email addresses (privacy-first)..."
EMAIL_PATTERN='[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}'
# Ignore local tool caches and third-party download trees (not project source).
EXCLUDE_PATTERN='example\.com|example\.org|test\.com|\.git|target|\.opencode|\.mimocode|\.cargo/'
PRIVACY_GREP_EXCLUDES=(
  --exclude-dir=.git
  --exclude-dir=target
  --exclude-dir=.cargo
  --exclude-dir=.opencode
  --exclude-dir=.mimocode
  --exclude-dir=node_modules
)

# Prefer git-tracked files so local caches never fail the template quality gate.
if git rev-parse --git-dir >/dev/null 2>&1; then
  if git ls-files -z | xargs -0 grep -nE "$EMAIL_PATTERN" 2>/dev/null | grep -vE "$EXCLUDE_PATTERN"; then
    fail "Email address detected in codebase"
  else
    pass "Privacy: OK"
  fi
elif grep -rE "$EMAIL_PATTERN" "${PRIVACY_GREP_EXCLUDES[@]}" . 2>/dev/null | grep -vE "$EXCLUDE_PATTERN"; then
  fail "Email address detected in codebase"
else
  pass "Privacy: OK"
fi
printf "\n"

# ============================================================
# 9. SECRET SCAN
# ============================================================
info "Scanning for potential secrets..."
SECRET_PATTERN="(api_key|token|secret|password|auth|key)[[:space:]]*[:=][[:space:]]*['\"][a-zA-Z0-9_\-]{16,}['\"]"
EXCLUDE_SECRET='example\.com|example\.org|test\.com|GITHUB_TOKEN|CARGO_REGISTRY_TOKEN|worktree'
SECRET_GREP_EXCLUDES=(
  --exclude-dir=.git
  --exclude-dir=target
  --exclude-dir=.cargo
  --exclude-dir=.agents
  --exclude-dir=.opencode
  --exclude-dir=node_modules
)

if git rev-parse --git-dir >/dev/null 2>&1; then
  if git ls-files -z | xargs -0 grep -nE "$SECRET_PATTERN" 2>/dev/null | grep -vE "$EXCLUDE_SECRET"; then
    fail "Potential secret detected in codebase"
  else
    pass "Secret Scan: OK"
  fi
elif grep -rE "$SECRET_PATTERN" "${SECRET_GREP_EXCLUDES[@]}" . 2>/dev/null | grep -vE "$EXCLUDE_SECRET"; then
  fail "Potential secret detected in codebase"
else
  pass "Secret Scan: OK"
fi
printf "\n"

# ============================================================
# 10. WORKFLOW VALIDATION
# ============================================================
info "Validating GitHub Actions workflows..."
if [[ -f "./scripts/validate-workflows.sh" ]]; then
  if ./scripts/validate-workflows.sh >/dev/null 2>&1; then
    pass "Workflows: valid"
  else
    warn "Workflows: validation issues (run ./scripts/validate-workflows.sh for details)"
  fi
else
  warn "validate-workflows.sh not found"
fi
printf "\n"

# ============================================================
# 11. SKILL EVALS
# ============================================================
info "Running skill evals..."
if [[ -f "./scripts/run-evals.sh" ]]; then
  if ./scripts/run-evals.sh >/dev/null 2>&1; then
    pass "Evals: all skills valid"
  else
    warn "Evals: issues found (run ./scripts/run-evals.sh --verbose for details)"
  fi
else
  warn "run-evals.sh not found"
fi
printf "\n"

# ============================================================
# 13. LLM CONTEXT FILES
# ============================================================
info "Checking LLM context files..."
if [[ -f "llms.txt" ]] && [[ -f "llms-full.txt" ]]; then
  pass "LLM context files present"
else
  warn "llms.txt or llms-full.txt missing (generate with ./scripts/generate-llms-txt.sh)"
fi
printf "\n"

# ============================================================
# 14. CI STATUS ARTIFACT
# ============================================================
info "Checking CI status artifact..."
if [[ -f ".agents/ci/ci-status.json" ]]; then
  if python3 -c "import json; json.load(open('.agents/ci/ci-status.json'))" 2>/dev/null; then
    pass "CI status artifact: valid JSON"
  else
    warn "CI status artifact: invalid JSON"
  fi
else
  warn "CI status artifact not found (.agents/ci/ci-status.json)"
fi
printf "\n"

# ============================================================
# 15. ROAST SCORER
# ============================================================
info "Running Roast Scorer..."
if [[ -f "./scripts/roast-scorer.sh" ]]; then
  if ./scripts/roast-scorer.sh; then
    pass "Roast Scorer: PASSED"
  else
    fail "Roast Scorer: FAILED (Score below threshold)"
  fi
else
  warn "Roast Scorer: scripts/roast-scorer.sh not found"
fi
printf "\n"

# ============================================================
# SUMMARY
# ============================================================
if [[ $FAILED -ne 0 ]]; then
  printf "${RED}─────────────────────────────────────────────────────────────────${NC}\n"
  printf "${RED}│ ✗ Quality Gate FAILED                                         │${NC}\n"
  printf "${RED}─────────────────────────────────────────────────────────────────${NC}\n"
  printf "\nLanguages checked: %s\n" "${DETECTED_LANGUAGES[*]}"
  exit 1
fi

printf "${GREEN}─────────────────────────────────────────────────────────────────${NC}\n"
printf "${GREEN}│ ✓ All Quality Gates PASSED                                    │${NC}\n"
printf "${GREEN}─────────────────────────────────────────────────────────────────${NC}\n"
printf "\nLanguages checked: %s\n" "${DETECTED_LANGUAGES[*]}"
