#!/usr/bin/env bash
# setup-hooks.sh: install Git hooks and commit template for this repo.
# Run once after cloning: bash scripts/setup-hooks.sh
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
HOOKS_DIR="$REPO_ROOT/hooks"
GIT_HOOKS_DIR="$REPO_ROOT/.git/hooks"

echo "Installing Git hooks from $HOOKS_DIR..."

for hook in commit-msg pre-push; do
  src="$HOOKS_DIR/$hook"
  dst="$GIT_HOOKS_DIR/$hook"
  if [ -f "$src" ]; then
    cp "$src" "$dst"
    chmod +x "$dst"
    echo "  Installed: $hook"
  else
    echo "  WARNING: $src not found, skipping."
  fi
done

echo "Setting commit template..."
git config commit.template "$REPO_ROOT/.gitmessage.txt"

echo ""
echo "Done. To also install pre-commit framework hooks, run:"
echo "  pre-commit install"
echo "  pre-commit install --hook-type commit-msg"
