#!/usr/bin/env bash
# Validates all CLI skill symlinks and SKILL.md files.
# Used in pre-commit hook and CI. Exit 2 on failure (surfaced to agent).
set +e
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SKILLS_SRC="$REPO_ROOT/.agents/skills"

CLI_SKILL_DIRS=(
  ".claude/skills"
  ".qwen/skills"
)

FAILED=0

# Colors (disabled in CI)
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

# Detect Windows (MSYS/Cygwin)
IS_WINDOWS=false
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" ]]; then
  IS_WINDOWS=true
fi

if [[ ! -d "$SKILLS_SRC" ]] || [[ -z "$(ls -A -- "$SKILLS_SRC" 2>/dev/null)" ]]; then
  echo "No skills in .agents/skills/ - nothing to validate."
  exit 0
fi

echo "Checking canonical skills and CLI symlinks..."

for skill_path in "$SKILLS_SRC"/*/; do
  [[ -d "$skill_path" ]] || continue
  skill_name="${skill_path%/}"
  skill_name="${skill_name##*/}"

  # Skip hidden/backup folders
  if [[ "$skill_name" == _* ]] || [[ "$skill_name" == .* ]]; then
    continue
  fi

  # Check 1: SKILL.md exists and has frontmatter
  skill_file="${skill_path}SKILL.md"
  if [[ ! -f "$skill_file" ]]; then
    printf "  ${RED}✗${NC} %s: Missing SKILL.md\n" "$skill_name"
    FAILED=1
    continue
  fi

  # Check frontmatter exists (starts with ---)
  if ! head -1 "$skill_file" | grep -q '^---'; then
    printf "  ${YELLOW}⚠${NC} %s: SKILL.md missing frontmatter (recommended: start with ---)\n" "$skill_name"
  else
    skill_lines=$(wc -l < "$skill_file")
    printf "  ${GREEN}✓${NC} %s: %s lines\n" "$skill_name" "$skill_lines"
  fi

  # Check 2: Circular symlink detection
  if [[ "$IS_WINDOWS" == "false" ]] && [[ -L "$skill_path" ]]; then
    printf "  ${RED}✗${NC} %s: Circular symlink detected\n" "$skill_name"
    FAILED=1
  fi

  # Check 3: Validate CLI symlinks
  for cli_dir in "${CLI_SKILL_DIRS[@]}"; do
    if [[ ! -d "$REPO_ROOT/$cli_dir" ]]; then
      continue
    fi

    link="$REPO_ROOT/$cli_dir/$skill_name"

    if [[ ! -L "$link" ]] && { [[ "$IS_WINDOWS" == "false" ]] || [[ ! -f "$link" ]]; }; then
      printf "  ${RED}✗${NC} MISSING symlink: %s/%s\n" "$cli_dir" "$skill_name"
      FAILED=1
    elif [[ ! -d "$link" ]] && { [[ "$IS_WINDOWS" == "false" ]] || [[ ! -f "$link" ]]; }; then
      printf "  ${RED}✗${NC} BROKEN symlink: %s/%s\n" "$cli_dir" "$skill_name"
      FAILED=1
    fi
  done
done

# Check 4: Skill authoring compliance
echo ""
echo "Checking skill authoring compliance..."

for skill_path in "$SKILLS_SRC"/*/; do
  [[ -d "$skill_path" ]] || continue
  skill_name="${skill_path%/}"
  skill_name="${skill_name##*/}"

  [[ "$skill_name" != _* ]] && [[ "$skill_name" != .* ]] || continue

  skill_file="${skill_path}SKILL.md"
  [[ -f "$skill_file" ]] || continue

  skill_failed=0

  # Check: frontmatter should contain name field (warn only for existing skills)
  has_name=$(awk '/^---$/{n++} n==1 && /^name:/{print "yes"; exit}' "$skill_file")
  if [[ -z "$has_name" ]]; then
    printf "  ${YELLOW}⚠${NC} %s: Missing 'name' field in frontmatter (recommended)\n" "$skill_name"
  fi

  # Check: frontmatter should contain category field (warn only)
  has_category=$(awk '/^---$/{n++} n==1 && /^category:/{print "yes"; exit}' "$skill_file")
  if [[ -z "$has_category" ]]; then
    printf "  ${YELLOW}⚠${NC} %s: Missing 'category' field in frontmatter (recommended)\n" "$skill_name"
  fi

  # Check: body must contain ## Rationalizations heading
  has_rationalizations=$(grep -c "^## Rationalizations" "$skill_file" || true)
  if [[ "$has_rationalizations" -eq 0 ]]; then
    printf "  ${YELLOW}⚠${NC} %s: Missing '## Rationalizations' section (recommended)\n" "$skill_name"
  fi

  # Check: body must contain ## Red Flags heading
  has_red_flags=$(grep -c "^## Red Flags" "$skill_file" || true)
  if [[ "$has_red_flags" -eq 0 ]]; then
    printf "  ${YELLOW}⚠${NC} %s: Missing '## Red Flags' section (recommended)\n" "$skill_name"
  fi

  # Check: evals/evals.json should exist (warn, not fail)
  evals_file="${skill_path}evals/evals.json"
  if [[ ! -f "$evals_file" ]]; then
    printf "  ${YELLOW}⚠${NC} %s: No evals/evals.json (recommended for skill validation)\n" "$skill_name"
  fi

  if [[ $skill_failed -ne 0 ]]; then
    FAILED=1
  fi
done

# Summary
echo ""
if [[ $FAILED -ne 0 ]]; then
  echo -e "${RED}─────────────────────────────────────────────────────────────────${NC}"
  echo -e "${RED}│ ✗ Skill Validation FAILED                                     │${NC}"
  echo -e "${RED}─────────────────────────────────────────────────────────────────${NC}"
  echo ""
  echo "Run: ./scripts/setup-skills.sh to fix missing symlinks."
  exit 2
fi

echo -e "${GREEN}─────────────────────────────────────────────────────────────────${NC}"
echo -e "${GREEN}│ ✓ All skills valid                                            │${NC}"
echo -e "${GREEN}─────────────────────────────────────────────────────────────────${NC}"
exit 0
