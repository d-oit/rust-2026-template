#!/usr/bin/env bash
# scripts/roast-scorer.sh
# Roast Scorer: Holistically evaluates the Rust project across 10 dimensions.
# Inspired by the web UI template, adapted for the Rust ecosystem.

set +e
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT" || exit 1

# --- Colors ---
if [[ -t 1 ]] && [[ "${FORCE_COLOR:-}" != "0" ]]; then
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  NC='\033[0m'
else
  RED=''
  GREEN=''
  NC=''
fi


# --- State ---
declare -A SCORES
declare -A REASONS
PASS_THRESHOLD=80
TOTAL_SCORE=0
REPORT_FILE="reports/roast-report.json"
mkdir -p reports

# ============================================================
# Dimension Implementations (Stubs for Step 1)
# ============================================================

score_dimension() {
  local id="$1"
  local name="$2"
  local score="$3"
  local reason="$4"
  SCORES[$id]=$score
  REASONS[$id]=$reason
  TOTAL_SCORE=$((TOTAL_SCORE + score))
}

# 1. Code Quality (10 pts)
check_code_quality() {
  local score=10
  local reasons=()

  # Clippy check (5 pts)
  if ! cargo clippy --all-targets --all-features -- -D warnings > /dev/null 2>&1; then
    score=$((score - 5))
    reasons+=("clippy warnings")
  fi

  # LOC check (5 pts)
  local MAX_LOC=500
  local violations=0
  while IFS= read -r file; do
    local lines
    lines=$(wc -l < "$file" 2>/dev/null || echo 0)
    if [[ "$lines" -gt "$MAX_LOC" ]]; then
      violations=$((violations + 1))
    fi
  done < <(find . -name "*.rs" -not -path "./target/*" -not -path "./.git/*" -type f 2>/dev/null)

  if [[ $violations -gt 0 ]]; then
    score=$((score - (violations > 5 ? 5 : violations)))
    reasons+=("$violations files > $MAX_LOC LOC")
  fi

  local reason="Zero clippy warnings, no files > $MAX_LOC LOC"
  [[ ${#reasons[@]} -gt 0 ]] && reason="Issues: $(IFS=,; echo "${reasons[*]}")"

  score_dimension "code_quality" "Code Quality" "$score" "$reason"
}

# 2. Test Coverage (10 pts)
check_test_coverage() {
  local score=0
  local reasons=()

  # Check for different test types (6 pts)
  if find tests -name "*_test.rs" -o -name "integration_test.rs" | grep -q .; then
    score=$((score + 2))
  else
    reasons+=("missing integration tests")
  fi

  if grep -r "\[test\]" src crates | grep -q .; then
    score=$((score + 2))
  else
    reasons+=("missing unit tests")
  fi

  if grep -r '/// ```rust' src crates | grep -q .; then
    score=$((score + 2))
  else
    reasons+=("missing doc tests")
  fi

  # Line coverage (4 pts)
  if command -v cargo-llvm-cov &>/dev/null; then
    local cov
    cov=$(cargo llvm-cov --workspace --all-features 2>/dev/null | grep "Total" | awk '{print $NF}' | tr -d '%')
    if [[ -n "$cov" ]]; then
      local cov_pts=$(( ${cov%.*} / 25 )) # 0-4 pts for 0-100%
      score=$((score + cov_pts))
      reasons+=("$cov% line coverage")
    fi
  else
    score=$((score + 2)) # Default 2 pts if tool not available but tests exist
    reasons+=("cargo-llvm-cov not installed")
  fi

  local reason="Excellent coverage across unit, integration, and doc tests"
  [[ ${#reasons[@]} -gt 0 ]] && reason="Summary: $(IFS=,; echo "${reasons[*]}")"

  score_dimension "test_coverage" "Test Coverage" "$score" "$reason"
}

# 3. Security (10 pts)
check_security() {
  local score=10
  local reasons=()

  # Cargo audit (4 pts)
  if command -v cargo-audit &>/dev/null; then
    if ! cargo audit > /dev/null 2>&1; then
      score=$((score - 4))
      reasons+=("vulnerabilities found")
    fi
  fi

  # Cargo deny (3 pts)
  if command -v cargo-deny &>/dev/null; then
    if ! cargo deny check advisories bans licenses > /dev/null 2>&1; then
      score=$((score - 3))
      reasons+=("deny violations")
    fi
  fi

  # Secret scan (3 pts)
  local SECRET_PATTERN="(api_key|token|secret|password|auth|key)[[:space:]]*[:=][[:space:]]*['\"][a-zA-Z0-9_\-]{16,}['\"]"
  local EXCLUDE_DIR='--exclude-dir=.git --exclude-dir=target --exclude-dir=.agents --exclude-dir=.opencode --exclude-dir=node_modules'
  local EXCLUDE_SECRET='example\.com|example\.org|test\.com|GITHUB_TOKEN|CARGO_REGISTRY_TOKEN|worktree'

  if grep -rE "$SECRET_PATTERN" $EXCLUDE_DIR . 2>/dev/null | grep -vE "$EXCLUDE_SECRET" | grep -q .; then
    score=$((score - 3))
    reasons+=("potential secrets")
  fi

  local reason="Zero audit findings, no secrets"
  [[ ${#reasons[@]} -gt 0 ]] && reason="Issues: $(IFS=,; echo "${reasons[*]}")"

  score_dimension "security" "Security" "$score" "$reason"
}

# 4. Dependency Health (10 pts)
check_dependency_health() {
  local score=10
  local reasons=()

  # Outdated dependencies (4 pts)
  if command -v cargo-outdated &>/dev/null; then
    local outdated
    outdated=$(cargo outdated --workspace --exit-code 1 2>/dev/null | wc -l)
    if [[ $outdated -gt 0 ]]; then
      score=$((score - 2))
      reasons+=("outdated deps")
    fi
  fi

  # MSRV compliance (3 pts)
  if [[ -f "./scripts/audit-msrv.sh" ]]; then
    if ! ./scripts/audit-msrv.sh > /dev/null 2>&1; then
      score=$((score - 3))
      reasons+=("MSRV non-compliant")
    fi
  fi

  # Crate layering (3 pts)
  if command -v cargo-deny &>/dev/null; then
    if ! cargo deny check sources > /dev/null 2>&1; then
      score=$((score - 3))
      reasons+=("layering violations")
    fi
  fi

  local reason="Zero outdated deps, MSRV compliant"
  [[ ${#reasons[@]} -gt 0 ]] && reason="Issues: $(IFS=,; echo "${reasons[*]}")"

  score_dimension "dependency_health" "Dependency Health" "$score" "$reason"
}

# 5. Documentation (10 pts)
check_documentation() {
  local score=10
  local reasons=()

  # Artifacts (4 pts)
  [[ ! -f "README.md" ]] && score=$((score - 1)) && reasons+=("missing README")
  [[ ! -f "AGENTS.md" ]] && score=$((score - 1)) && reasons+=("missing AGENTS.md")
  [[ ! -d "plans/adr" ]] && score=$((score - 1)) && reasons+=("missing ADRs")
  [[ ! -f "CONTRIBUTING.md" ]] && score=$((score - 1)) && reasons+=("missing CONTRIBUTING.md")

  # Doc comments (6 pts)
  local total_public
  total_public=$(grep -r "pub " src crates --exclude-dir=target | wc -l)
  local doc_comments
  doc_comments=$(grep -r "///" src crates --exclude-dir=target | wc -l)

  if [[ $total_public -gt 0 ]]; then
    if [[ $doc_comments -lt $total_public ]]; then
      score=$((score - 3))
      reasons+=("missing doc comments")
    fi
  fi

  local reason="README, AGENTS.md, ADRs present; public items documented"
  [[ ${#reasons[@]} -gt 0 ]] && reason="Issues: $(IFS=,; echo "${reasons[*]}")"

  score_dimension "documentation" "Documentation" "$score" "$reason"
}

# 6. Architecture (10 pts)
check_architecture() {
  local score=10
  local reasons=()

  # Fitness check (5 pts)
  if ! cargo test --test arch_fitness > /dev/null 2>&1; then
    score=$((score - 5))
    reasons+=("fitness tests failed")
  fi

  # Crate layering (5 pts) - check if deny.toml exists as a proxy for rules
  if [[ ! -f "deny.toml" ]]; then
    score=$((score - 2))
    reasons+=("missing deny.toml")
  fi

  local reason="Fitness functions pass, layering valid"
  [[ ${#reasons[@]} -gt 0 ]] && reason="Issues: $(IFS=,; echo "${reasons[*]}")"

  score_dimension "architecture" "Architecture" "$score" "$reason"
}

# 7. Build Health (10 pts)
check_build_health() {
  local score=10
  local reasons=()

  # Clean build check (10 pts)
  if ! cargo build --workspace > /dev/null 2>&1; then
    score=0
    reasons+=("build failed")
  else
    # Check for build warnings
    if cargo build --workspace 2>&1 | grep -q "warning:"; then
      score=$((score - 5))
      reasons+=("build warnings")
    fi
  fi

  local reason="Clean build, no warnings"
  [[ ${#reasons[@]} -gt 0 ]] && reason="Issues: $(IFS=,; echo "${reasons[*]}")"

  score_dimension "build_health" "Build Health" "$score" "$reason"
}

# 8. Performance (10 pts)
check_performance() {
  local score=10
  local reasons=()

  # Benchmarks compile (5 pts)
  if ! cargo bench --workspace --no-run > /dev/null 2>&1; then
    score=$((score - 5))
    reasons+=("benchmarks don't compile")
  fi

  # Profile optimization (5 pts)
  if ! grep -q "\[profile.release\]" Cargo.toml; then
    score=$((score - 5))
    reasons+=("missing release profile")
  fi

  local reason="Benchmarks compile, release profile optimized"
  [[ ${#reasons[@]} -gt 0 ]] && reason="Issues: $(IFS=,; echo "${reasons[*]}")"

  score_dimension "performance" "Performance" "$score" "$reason"
}

# 9. Agent Readiness (10 pts)
check_agent_readiness() {
  local score=10
  local reasons=()

  # Skills (4 pts)
  if [[ ! -d ".agents/skills" ]] || [[ -z "$(ls -A .agents/skills 2>/dev/null)" ]]; then
    score=$((score - 4))
    reasons+=("missing skills")
  fi

  # Context (3 pts)
  if [[ ! -f "llms.txt" ]]; then
    score=$((score - 3))
    reasons+=("missing llms.txt")
  fi

  # Contract (3 pts)
  if ! grep -q "Agent Coding Contract" AGENTS.md 2>/dev/null; then
    score=$((score - 3))
    reasons+=("AGENTS.md incomplete")
  fi

  local reason="All agent artifacts present and valid"
  [[ ${#reasons[@]} -gt 0 ]] && reason="Issues: $(IFS=,; echo "${reasons[*]}")"

  score_dimension "agent_readiness" "Agent Readiness" "$score" "$reason"
}

# 10. Release Readiness (10 pts)
check_release_readiness() {
  local score=10
  local reasons=()

  # Version check (4 pts)
  if [[ -f "./scripts/propagate-version.sh" ]]; then
    if ! ./scripts/propagate-version.sh --check > /dev/null 2>&1; then
      score=$((score - 4))
      reasons+=("VERSION mismatch")
    fi
  fi

  # Changelog (3 pts)
  if [[ ! -f "CHANGELOG.md" ]] || [[ ! -s "CHANGELOG.md" ]]; then
    score=$((score - 3))
    reasons+=("missing or empty CHANGELOG.md")
  fi

  # Conventional commits (3 pts)
  if [[ -f "commitlint.config.cjs" ]]; then
    # Simple proxy: check if we have a git log that looks conventional
    if ! git log -n 5 --pretty=format:%s | grep -qE "^(feat|fix|docs|style|refactor|perf|test|chore|ci|build|revert)(\(.+\))?!?: .+$"; then
      score=$((score - 1))
      reasons+=("some commits non-conventional")
    fi
  else
     score=$((score - 3))
     reasons+=("missing commitlint config")
  fi

  local reason="VERSION matches, CHANGELOG present, commits conventional"
  [[ ${#reasons[@]} -gt 0 ]] && reason="Issues: $(IFS=,; echo "${reasons[*]}")"

  score_dimension "release_readiness" "Release Readiness" "$score" "$reason"
}

# ============================================================
# Main Execution
# ============================================================

printf "=== Roast Scorer (rust-2026-template) ===\n\n"

check_code_quality
check_test_coverage
check_security
check_dependency_health
check_documentation
check_architecture
check_build_health
check_performance
check_agent_readiness
check_release_readiness

# Sort keys for consistent output
KEYS=("code_quality" "test_coverage" "security" "dependency_health" "documentation" "architecture" "build_health" "performance" "agent_readiness" "release_readiness")
NAMES=("Code Quality" "Test Coverage" "Security" "Dependency Health" "Documentation" "Architecture" "Build Health" "Performance" "Agent Readiness" "Release Readiness")

for i in "${!KEYS[@]}"; do
  key=${KEYS[$i]}
  name=${NAMES[$i]}
  printf "%2d. %-20s %2d/10 — %s\n" "$((i+1))" "$name" "${SCORES[$key]}" "${REASONS[$key]}"
done

printf "\nTotal: %d/100 — " "$TOTAL_SCORE"
if [[ $TOTAL_SCORE -ge $PASS_THRESHOLD ]]; then
  printf "${GREEN}✅ PASS${NC}\n"
  EXIT_CODE=0
else
  printf "${RED}❌ FAIL${NC} (Minimum 80 required)\n"
  EXIT_CODE=1
fi

# ============================================================
# JSON Output (No jq required)
# ============================================================

{
  echo "{"
  echo "  \"timestamp\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
  echo "  \"total_score\": $TOTAL_SCORE,"
  echo "  \"pass_threshold\": $PASS_THRESHOLD,"
  echo "  \"pass\": $([[ $TOTAL_SCORE -ge $PASS_THRESHOLD ]] && echo "true" || echo "false"),"
  echo "  \"dimensions\": {"
  for i in "${!KEYS[@]}"; do
    key=${KEYS[$i]}
    name=${NAMES[$i]}
    printf "    \"%s\": { \"name\": \"%s\", \"score\": %d, \"reason\": \"%s\" }%s\n" \
      "$key" "$name" "${SCORES[$key]}" "${REASONS[$key]}" "$([[ $i -lt 9 ]] && echo "," || echo "")"
  done
  echo "  }"
  echo "}"
} > "$REPORT_FILE"

exit $EXIT_CODE
