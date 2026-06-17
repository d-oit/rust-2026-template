#!/usr/bin/env bash
# Creates a single symlink from CLI-specific folders -> .agents/skills/ (canonical source)
# Run once after cloning: ./scripts/setup-skills.sh
# Note: OpenCode reads skills directly from .agents/skills/ - no symlinks needed.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.."; pwd)"
SKILLS_SRC="$REPO_ROOT/.agents/skills"

CLI_SKILL_DIRS=(
  ".claude/skills"
  ".qwen/skills"
)

if [[ ! -d "$SKILLS_SRC" ]]; then
  printf "No skills found at .agents/skills/ - nothing to symlink.\n"
  exit 0
fi

printf "Setting up skill symlinks from .agents/skills/...\n"

for cli_dir in "${CLI_SKILL_DIRS[@]}"; do
  target_dir="$REPO_ROOT/$cli_dir"

  # If it's already a symlink, skip
  if [[ -L "$target_dir" ]]; then
    current_target=$(readlink "$target_dir")
    if [[ "$current_target" == *"agents/skills"* ]]; then
      printf "  skip (exists): %s -> %s\n" "$cli_dir" "$current_target"
      continue
    fi
  fi

  # If it's a real directory, remove it (it contains old individual symlinks)
  if [[ -d "$target_dir" ]] && [[ ! -L "$target_dir" ]]; then
    # Check if it contains only symlinks (old-style setup)
    has_real_files=false
    for item in "$target_dir"/*; do
      if [[ -e "$item" ]] && [[ ! -L "$item" ]]; then
        has_real_files=true
        break
      fi
    done

    if [[ "$has_real_files" == "false" ]]; then
      # Only contains symlinks or is empty - safe to replace
      rm -rf "$target_dir"
    else
      printf "  WARN: %s contains real files - skipping\n" "$cli_dir"
      continue
    fi
  fi

  # Create parent directory if needed
  mkdir -p -- "$(dirname "$target_dir")"

  # Create the single symlink
  rel_base=$(realpath --relative-to="$(dirname "$target_dir")" "$SKILLS_SRC")
  ln -s -- "$rel_base" "$target_dir"
  printf "  linked: %s -> %s\n" "$cli_dir" "$rel_base"
done

printf "\n"
printf "Skill symlinks created. Run scripts/validate-skills.sh to verify.\n"
