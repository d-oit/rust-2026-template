#!/usr/bin/env bash
# sync-architecture.sh - Regenerate architecture SVG and sync to docs
# Idempotent: safe to re-run.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

log()  { printf '==> %s\n' "$*"; }
ok()   { printf '  \033[0;32m✓\033[0m %s\n' "$*"; }
warn() { printf '  ! %s\n' "$*"; }

# --- Generate SVG ---
SVG_GEN=".agents/skills/architecture-diagram/scripts/generate_diagram.py"
SVG_OUT=".template/architecture.svg"

if [[ -f "$SVG_GEN" ]]; then
  log "Generating architecture diagram"
  python3 "$SVG_GEN" --root . --out "$SVG_OUT"
  ok "Generated $SVG_OUT"
else
  warn "$SVG_GEN not found - skipping generation"
fi

# --- Sync to docs ---
if [[ -f "$SVG_OUT" ]]; then
  mkdir -p docs/src
  cp "$SVG_OUT" docs/src/architecture.svg
  ok "Synced to docs/src/architecture.svg"
fi

log "Done"
