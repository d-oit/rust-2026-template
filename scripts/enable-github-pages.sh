#!/usr/bin/env bash
# enable-github-pages.sh - Enable GitHub Pages for this repository
# Requires: gh CLI authenticated
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

log()  { printf '==> %s\n' "$*"; }
ok()   { printf '  \033[0;32m✓\033[0m %s\n' "$*"; }
warn() { printf '  ! %s\n' "$*"; }

# Check gh CLI
if ! command -v gh &>/dev/null; then
  warn "gh CLI not found. Install: https://cli.github.com/"
  exit 1
fi

# Check if authenticated
if ! gh auth status &>/dev/null; then
  warn "Not authenticated. Run: gh auth login"
  exit 1
fi

REPO=$(gh repo view --json nameWithOwner -q '.nameWithOwner')
log "Repository: $REPO"

# Enable GitHub Pages with GitHub Actions source
log "Enabling GitHub Pages..."
gh api -X PUT "repos/${REPO}/pages" \
  -f source='{"branch":"main","path":"/docs/book"}' 2>/dev/null || \
gh api -X POST "repos/${REPO}/pages" \
  -f build_type='legacy' \
  -f source='{"branch":"main","path":"/docs/book"}' 2>/dev/null || true

ok "GitHub Pages configured"

# Check deployment status
log "Checking deployment..."
DEPLOY_URL="https://$(gh api repos/${REPO} -q '.owner.login').github.io/$(gh api repos/${REPO} -q '.name')"
ok "Deploy URL: ${DEPLOY_URL}"

echo ""
echo "To deploy manually:"
echo "  gh workflow run deploy-docs.yml"
echo ""
echo "Or push changes to docs/ to trigger automatic deployment."
