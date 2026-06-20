#!/usr/bin/env bash
# setup-mdbook.sh - Set up mdbook documentation and integrate architecture SVG
# Idempotent: safe to re-run.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

log()  { printf '==> %s\n' "$*"; }
ok()   { printf '  \033[0;32m✓\033[0m %s\n' "$*"; }
warn() { printf '  ! %s\n' "$*"; }

# --- Check mdbook ---
if ! command -v mdbook &>/dev/null; then
  # Try ~/.cargo/bin first
  if [[ -x "$HOME/.cargo/bin/mdbook" ]]; then
    MDBOOK="$HOME/.cargo/bin/mdbook"
  else
    log "Installing mdbook..."
    curl -sSL https://github.com/rust-lang/mdBook/releases/download/v0.4.40/mdbook-v0.4.40-x86_64-unknown-linux-gnu.tar.gz | tar xz -C /tmp
    mkdir -p "$HOME/.cargo/bin"
    mv /tmp/mdbook "$HOME/.cargo/bin/"
    MDBOOK="$HOME/.cargo/bin/mdbook"
    ok "mdbook installed"
  fi
else
  MDBOOK="$(command -v mdbook)"
fi

log "Using: $MDBOOK ($($MDBOOK --version 2>/dev/null || echo 'unknown version'))"

# --- Create directory structure ---
mkdir -p docs/src

# --- book.toml ---
if [[ ! -f docs/book.toml ]]; then
  cat > docs/book.toml << 'TOML'
[book]
title = "Rust 2026 Template"
authors = ["Maintainer"]
language = "en"
src = "src"

[output.html]
default-theme = "light"
TOML
  ok "Created docs/book.toml"
else
  ok "docs/book.toml exists"
fi

# --- SUMMARY.md ---
if [[ ! -f docs/src/SUMMARY.md ]]; then
  cat > docs/src/SUMMARY.md << 'MD'
# Summary

- [Overview](./README.md)
- [Getting Started](./getting-started.md)
- [Architecture](./architecture.md)
MD
  ok "Created docs/src/SUMMARY.md"
else
  ok "docs/src/SUMMARY.md exists"
fi

# --- README.md ---
if [[ ! -f docs/src/README.md ]]; then
  cat > docs/src/README.md << 'MD'
# Rust 2026 Template

A production-ready Rust workspace template with modern tooling and AI agent integration.

## Quick Start

```bash
./scripts/bootstrap.sh
cargo build --workspace
```
MD
  ok "Created docs/src/README.md"
else
  ok "docs/src/README.md exists"
fi

# --- Copy SVG ---
SVG_SRC=".template/architecture.svg"
SVG_DST="docs/src/architecture.svg"

if [[ -f "$SVG_SRC" ]]; then
  cp "$SVG_SRC" "$SVG_DST"
  ok "Copied $SVG_SRC → $SVG_DST"
else
  warn "$SVG_SRC not found - run architecture diagram generator first"
fi

# --- Sync existing docs into src/ ---
for f in docs/architecture.md docs/ci.md docs/dora-metrics.md; do
  if [[ -f "$f" ]]; then
    basename="$(basename "$f")"
    dst="docs/src/$basename"
    if [[ ! -f "$dst" ]]; then
      cp "$f" "$dst"
      ok "Synced $f → $dst"
    fi
  fi
done

log "Setup complete. Run: $MDBOOK serve docs"
