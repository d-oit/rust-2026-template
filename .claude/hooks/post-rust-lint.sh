#!/usr/bin/env bash
# PostToolUse hook: auto-format and lint after any file write/edit.
# Runs rustfmt + cargo clippy and injects diagnostics as additionalContext
# so Claude sees and fixes issues in the same turn rather than the next.
set -euo pipefail

input="$(cat)"
file="$(echo "$input" | jq -r '.tool_input.file_path // .tool_input.path // empty' 2>/dev/null || true)"

# Only act on Rust source files
case "$file" in
  *.rs) ;;
  *) exit 0 ;;
esac

# Format silently — don't fail the hook on format errors
rustfmt "$file" >/dev/null 2>&1 || true

# Run Clippy and capture output (limit to 40 lines to stay within context)
diag="$(cargo clippy --quiet 2>&1 | grep -E '^(error|warning)' | head -40 || true)"

if [ -n "$diag" ]; then
  jq -Rn --arg msg "$diag" '{
    hookSpecificOutput: {
      hookEventName: "PostToolUse",
      additionalContext: $msg
    }
  }'
fi
