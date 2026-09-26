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

# Body budget for a single SKILL.md, excluding frontmatter.
# The Agent Skills spec recommends <500 lines; this repo's skill-creator sets
# the tighter 250-line budget so the always-loaded instruction set stays small.
MAX_SKILL_BODY_LINES=250

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

  # Skip hidden/backup folders and drafts directory
  if [[ "$skill_name" == _* ]] || [[ "$skill_name" == .* ]] || [[ "$skill_name" == "drafts" ]]; then
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

  # Check 3: Validate CLI folder symlinks (single symlink per CLI dir)
  for cli_dir in "${CLI_SKILL_DIRS[@]}"; do
    cli_full="$REPO_ROOT/$cli_dir"

    # Skip if CLI dir doesn't exist at all
    if [[ ! -e "$cli_full" ]] && [[ ! -L "$cli_full" ]]; then
      continue
    fi

    # Check that the CLI dir is a symlink to .agents/skills
    if [[ -L "$cli_full" ]]; then
      link_target=$(readlink "$cli_full")
      if [[ "$link_target" != *"agents/skills"* ]]; then
        printf "  ${RED}✗${NC} %s is a symlink but points to %s (expected .agents/skills)\n" "$cli_dir" "$link_target"
        FAILED=1
      fi
    elif [[ -d "$cli_full" ]]; then
      # It's a real directory - check if it contains individual symlinks (old-style)
      has_individual_links=false
      for item in "$cli_full"/*; do
        if [[ -L "$item" ]]; then
          has_individual_links=true
          break
        fi
      done

      if [[ "$has_individual_links" == "true" ]]; then
        printf "  ${YELLOW}⚠${NC} %s is a real directory with individual symlinks (consider running setup-skills.sh to switch to folder symlink)\n" "$cli_dir"
      fi
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

  [[ "$skill_name" != _* ]] && [[ "$skill_name" != .* ]] && [[ "$skill_name" != "drafts" ]] || continue

  skill_file="${skill_path}SKILL.md"
  [[ -f "$skill_file" ]] || continue

  skill_failed=0

  # Hard check: frontmatter `name` must exist and match the directory name.
  # REQUIRED by the Agent Skills spec; packaging hard-fails without it.
  fm_name=$(awk '/^---$/{n++} n==1 && /^name:/{sub(/^name:[[:space:]]*/,""); gsub(/"/,""); print; exit}' "$skill_file")
  if [[ -z "$fm_name" ]]; then
    printf "  ${RED}✗${NC} %s: Missing 'name' in frontmatter (spec-required)\n" "$skill_name"
    skill_failed=1
  elif [[ "$fm_name" != "$skill_name" ]]; then
    printf "  ${RED}✗${NC} %s: name '%s' must match directory name (spec-required)\n" "$skill_name" "$fm_name"
    skill_failed=1
  fi

  # Hard check: frontmatter limited to the six portable spec fields.
  non_spec=$(awk '/^---$/{n++} n==1 && /^[a-zA-Z_-]+:/{sub(/:.*/,""); print}' "$skill_file" \
    | grep -vxE 'name|description|license|compatibility|metadata|allowed-tools' || true)
  if [[ -n "$non_spec" ]]; then
    printf "  ${RED}✗${NC} %s: Non-spec frontmatter key(s): %s (use metadata.* instead)\n" \
      "$skill_name" "$(echo "$non_spec" | tr '\n' ' ')"
    skill_failed=1
  fi

  # Hard check: description present and within the 1024-character spec limit.
  fm_desc=$(awk '/^---$/{n++} n==1 && /^description:/{sub(/^description:[[:space:]]*/,""); print; exit}' "$skill_file")
  if [[ -z "$fm_desc" ]]; then
    printf "  ${RED}✗${NC} %s: Missing 'description' in frontmatter (spec-required)\n" "$skill_name"
    skill_failed=1
  fi

  # Hard check: SKILL.md stays within the documented budget.
  body_lines=$(awk 'NR==1 && /^---[[:space:]]*$/ {fm=1; next} fm && !seen && /^---[[:space:]]*$/ {seen=1; next} seen {c++} END {print c+0}' "$skill_file")
  if [[ -n "$body_lines" && "$body_lines" -gt "$MAX_SKILL_BODY_LINES" ]]; then
    printf "  ${RED}✗${NC} %s: body is %s lines (max %s)\n" "$skill_name" "$body_lines" "$MAX_SKILL_BODY_LINES"
    skill_failed=1
  fi

  # Warning: frontmatter should contain a category (spec-legal only under metadata)
  has_category=$(awk '/^---$/{n++} n==1 && /^  ?category:/{print "yes"; exit}' "$skill_file")
  if [[ -z "$has_category" ]]; then
    printf "  ${YELLOW}⚠${NC} %s: Missing 'metadata.category' in frontmatter (recommended)\n" "$skill_name"
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
