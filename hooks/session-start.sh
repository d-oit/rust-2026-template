#!/usr/bin/env bash
# SessionStart hook — injects project doc context into agent sessions (read-only)
set -euo pipefail

DOCS_ROOT="${DOCS_ROOT:-docs}"
CHANGELOG="${CHANGELOG:-CHANGELOG.md}"

echo "=== Project Context ==="
echo "Docs root : $DOCS_ROOT"

# Print crate structure
if [ -f "Cargo.toml" ]; then
  echo "--- Cargo.toml (workspace/package) ---"
  grep -E '^(name|version|\[workspace\]|members)' Cargo.toml | head -20
fi

# Print workspace members
if [ -f "Cargo.toml" ]; then
  echo "--- Workspace Members ---"
  awk '
    /^\[workspace\]/ { in_ws=1; next }
    /^\[/ { in_ws=0; next }
    in_ws && /^members/ {
      if (match($0, /\[.*\]/)) {
        gsub(/[\[\]"'\'',]/, " ")
        print
        next
      }
      in_members=1
      next
    }
    in_members && /^\]/ { in_members=0; next }
    in_members { gsub(/[[:space:]]*["'\'',]/, " "); print }
  ' Cargo.toml | tr -s ' ' | sed 's/^ *//' | head -20
fi

# Print doc structure map
if [ -d "$DOCS_ROOT" ]; then
  echo "--- Docs Map ---"
  find "$DOCS_ROOT" -maxdepth 2 -type f -name '*.md' | sort
fi

# Print latest changelog entry
if [ -f "$CHANGELOG" ]; then
  echo "--- Latest Changelog Entry ---"
  awk '/^## /{count++; if(count==2) exit} count==1{print}' "$CHANGELOG"
fi

# Print CI health status
if [ -f ".agents/ci/ci-status.json" ]; then
  echo "--- CI Status ---"
  if command -v python3 &>/dev/null; then
    python3 -c "
import json
with open('.agents/ci/ci-status.json') as f:
    data = json.load(f)
print(f\"Timestamp: {data.get('timestamp', 'unknown')}\")
print(f\"Commit: {data.get('commit', 'unknown')[:8]}\")
print(f\"Branch: {data.get('branch', 'unknown')}\")
print(f\"Overall: {data.get('overall', 'unknown')}\")
# Support both legacy jobs map and current checks array formats
failed = []
if 'jobs' in data:
    failed = [k for k, v in data['jobs'].items() if v == 'failure']
elif 'checks' in data:
    failed = [c['name'] for c in data['checks'] if c.get('status') == 'failure']
if failed:
    print(f'Failed: {\", \".join(failed)}')
" 2>/dev/null || echo "  (Could not parse CI status)"
  fi
fi

# Print skill count
if [ -d ".agents/skills" ]; then
  SKILL_COUNT=$(find .agents/skills -maxdepth 1 -mindepth 1 -type d | wc -l)
  echo "--- Skills: $SKILL_COUNT available ---"
fi

# Print active workflow state
if [ -f ".agents/context/workflow-state.json" ]; then
  echo "--- Workflow State ---"
  if command -v python3 &>/dev/null; then
    python3 -c "
import json
with open('.agents/context/workflow-state.json') as f:
    data = json.load(f)
task = data.get('current_task', 'none')
agent = data.get('assigned_to', 'none')
print(f\"Current task: {task}\")
print(f\"Assigned to: {agent}\")
" 2>/dev/null || echo "  (Could not parse workflow state)"
  fi
fi

echo "====================="
