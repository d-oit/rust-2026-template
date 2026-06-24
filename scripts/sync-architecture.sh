#!/usr/bin/env bash
# sync-architecture.sh - Regenerate diagrams and sync to docs
# Idempotent: safe to re-run.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

log()  { printf '==> %s\n' "$*"; }
ok()   { printf '  \033[0;32m✓\033[0m %s\n' "$*"; }
warn() { printf '  ! %s\n' "$*"; }

# --- Generate architecture diagram ---
GEN_SCRIPT=".agents/skills/architecture-diagram/scripts/generate_diagram.py"
if [[ -f "$GEN_SCRIPT" ]]; then
  log "Generating architecture diagram"
  python3 "$GEN_SCRIPT" --root . \
    --out .template/architecture.excalidraw \
    --svg-out .template/architecture.svg \
    --png-out .template/architecture.png
  ok "Generated architecture.{excalidraw,svg,png}"
else
  warn "$GEN_SCRIPT not found - skipping"
fi

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

# --- Sync artifacts to docs ---
mkdir -p docs/src
for f in architecture overview; do
  for ext in svg png; do
    src=".template/${f}.${ext}"
    dst="docs/src/${f}.${ext}"
    if [[ -f "$src" ]]; then
      cp "$src" "$dst"
      ok "Synced to $dst"
    fi
  done
done

log "Done"
