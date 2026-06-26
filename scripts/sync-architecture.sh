#!/usr/bin/env bash
# sync-architecture.sh - Regenerate overview diagram and sync to docs
# Idempotent: safe to re-run.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

log()  { printf '==> %s\n' "$*"; }
ok()   { printf '  \033[0;32m✓\033[0m %s\n' "$*"; }
warn() { printf '  ! %s\n' "$*"; }

# --- Generate overview infographic ---
OVERVIEW_SCRIPT=".agents/skills/architecture-diagram/scripts/generate_overview.py"
if [[ -f "$OVERVIEW_SCRIPT" ]]; then
  log "Generating overview infographic"
  python3 "$OVERVIEW_SCRIPT" --root . \
    --out .template/overview.excalidraw \
    --svg-out .template/overview.svg \
    --png-out .template/overview.png
  ok "Generated overview.{excalidraw,svg,png}"
else
  warn "$OVERVIEW_SCRIPT not found - skipping"
fi

# --- Sync SVGs to docs ---
mkdir -p docs/src
src=".template/overview.svg"
dst="docs/src/overview.svg"
if [[ -f "$src" ]]; then
  cp "$src" "$dst"
  ok "Synced to $dst"
fi

log "Done"
