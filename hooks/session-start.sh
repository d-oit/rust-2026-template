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

echo "====================="
