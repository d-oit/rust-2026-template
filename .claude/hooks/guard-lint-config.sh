#!/usr/bin/env bash
# PreToolUse hook: block AI agent edits to lint/quality-gate config files.
# Agents hitting a Clippy/fmt error should fix the code, not silence the tool.
# If a lint config change is genuinely needed, a human must make it manually.
set -euo pipefail

input="$(cat)"
file="$(echo "$input" | jq -r '.tool_input.file_path // .tool_input.path // empty' 2>/dev/null || true)"

# Protected files — changes require human review
PROTECTED=(
  "Cargo.toml"
  ".clippy.toml"
  "rustfmt.toml"
  ".pre-commit-config.yaml"
  "deny.toml"
  ".gitleaks.toml"
  ".yamllint.yml"
  ".shellcheckrc"
)

basename="$(basename "$file")"

for protected in "${PROTECTED[@]}"; do
  if [ "$basename" = "$protected" ]; then
    jq -n --arg f "$file" '{
      decision: "block",
      reason: ("Editing lint/quality-gate config files is not permitted for AI agents. Fix the code instead of silencing the linter. File: " + $f)
    }'
    exit 0
  fi
done
