#!/usr/bin/env bash
# sync-architecture.sh - Regenerate architecture diagram and sync to docs
# Idempotent: safe to re-run.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

log()  { printf '==> %s\n' "$*"; }
ok()   { printf '  \033[0;32m✓\033[0m %s\n' "$*"; }
warn() { printf '  ! %s\n' "$*"; }

# --- Generate Excalidraw + SVG ---
GEN_SCRIPT=".agents/skills/architecture-diagram/scripts/generate_diagram.py"
EXCALIDRAW_OUT=".template/architecture.excalidraw"
SVG_OUT=".template/architecture.svg"

if [[ -f "$GEN_SCRIPT" ]]; then
  log "Generating architecture diagram"
  python3 "$GEN_SCRIPT" --root . --out "$EXCALIDRAW_OUT" --svg-out "$SVG_OUT"
  ok "Generated $EXCALIDRAW_OUT and $SVG_OUT"
else
  warn "$GEN_SCRIPT not found - skipping generation"
fi

# --- Sync SVG to docs ---
if [[ -f "$SVG_OUT" ]]; then
  mkdir -p docs/src
  cp "$SVG_OUT" docs/src/architecture.svg
  ok "Synced to docs/src/architecture.svg"
fi

log "Done"
