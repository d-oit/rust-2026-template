#!/usr/bin/env bash
# scripts/run-evals.sh
# Discovers and runs skill evals, producing a summary report.
# Usage: ./scripts/run-evals.sh [--skill <name>] [--verbose]
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

# --- Parse arguments ---
SKILL_FILTER=""
VERBOSE=false
for arg in "$@"; do
  case $arg in
    --skill) shift; SKILL_FILTER="${1:-}"; shift ;;
    --skill=*) SKILL_FILTER="${arg#*=}" ;;
    --verbose) VERBOSE=true ;;
    *) echo "Unknown argument: $arg"; exit 1 ;;
  esac
done

# --- Find evals ---
SKILLS_DIR=".agents/skills"
if [[ ! -d "$SKILLS_DIR" ]]; then
  echo -e "${RED}[ERROR]${NC} Skills directory not found: $SKILLS_DIR"
  exit 1
fi

TOTAL=0
PASSED=0
FAILED=0
SKIPPED=0
RESULTS=()

for eval_file in "$SKILLS_DIR"/*/evals/evals.json; do
  [[ -f "$eval_file" ]] || continue

  SKILL_NAME=$(basename "$(dirname "$(dirname "$eval_file")")")

  # Apply filter
  if [[ -n "$SKILL_FILTER" && "$SKILL_NAME" != "$SKILL_FILTER" ]]; then
    continue
  fi

  TOTAL=$((TOTAL + 1))

  # Parse evals.json
  if ! command -v python3 &>/dev/null; then
    echo -e "${YELLOW}[WARN]${NC} python3 not found, skipping eval validation"
    SKIPPED=$((SKIPPED + 1))
    continue
  fi

  EVAL_DATA=$(python3 -c "
import json, sys
with open('$eval_file') as f:
    data = json.load(f)

# Handle both array format and object format with 'evals' key
evals = data if isinstance(data, list) else data.get('evals', [])
print(len(evals))
for e in evals:
    eid = e.get('id', e.get('name', '?'))
    prompt = e.get('prompt', e.get('input', ''))[:80]
    assertions = e.get('assertions', [])
    print(f'{eid}|{prompt}|{len(assertions)}')
" 2>/dev/null) || {
    echo -e "${RED}[FAIL]${NC} $SKILL_NAME: invalid evals.json"
    FAILED=$((FAILED + 1))
    RESULTS+=("$SKILL_NAME|FAIL|invalid JSON")
    continue
  }

  EVAL_COUNT=$(echo "$EVAL_DATA" | head -1)
  if [[ "$EVAL_COUNT" -eq 0 ]]; then
    if $VERBOSE; then
      echo -e "${YELLOW}[SKIP]${NC} $SKILL_NAME: no evals defined"
    fi
    SKIPPED=$((SKIPPED + 1))
    RESULTS+=("$SKILL_NAME|SKIP|no evals")
    continue
  fi

  # Validate structure of each eval
  SKILL_OK=true
  while IFS='|' read -r eid prompt assert_count; do
    [[ "$eid" == "^[0-9]*$" ]] 2>/dev/null || true
    if [[ -z "$eid" || -z "$prompt" ]]; then
      echo -e "${RED}[FAIL]${NC} $SKILL_NAME eval #$eid: missing required fields"
      SKILL_OK=false
    fi
    if [[ "$assert_count" -eq 0 ]]; then
      echo -e "${YELLOW}[WARN]${NC} $SKILL_NAME eval #$eid: no assertions defined"
    fi
  done < <(echo "$EVAL_DATA" | tail -n +2)

  if $SKILL_OK; then
    PASSED=$((PASSED + 1))
    if $VERBOSE; then
      echo -e "${GREEN}[PASS]${NC} $SKILL_NAME: $EVAL_COUNT evals, valid structure"
    fi
    RESULTS+=("$SKILL_NAME|PASS|$EVAL_COUNT evals")
  else
    FAILED=$((FAILED + 1))
    RESULTS+=("$SKILL_NAME|FAIL|structure errors")
  fi
done

# --- Summary ---
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Skill Eval Summary"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
printf "  Total:   %d\n" "$TOTAL"
printf "  ${GREEN}Passed:  %d${NC}\n" "$PASSED"
if [[ $FAILED -gt 0 ]]; then
  printf "  ${RED}Failed:  %d${NC}\n" "$FAILED"
fi
if [[ $SKIPPED -gt 0 ]]; then
  printf "  ${YELLOW}Skipped: %d${NC}\n" "$SKIPPED"
fi
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if $VERBOSE; then
  echo ""
  echo "Detailed Results:"
  for result in "${RESULTS[@]}"; do
    IFS='|' read -r name status detail <<< "$result"
    case $status in
      PASS) printf "  ${GREEN}✓${NC} %-25s %s\n" "$name" "$detail" ;;
      FAIL) printf "  ${RED}✗${NC} %-25s %s\n" "$name" "$detail" ;;
      SKIP) printf "  ${YELLOW}○${NC} %-25s %s\n" "$name" "$detail" ;;
    esac
  done
fi

if [[ $FAILED -gt 0 ]]; then
  exit 1
fi
