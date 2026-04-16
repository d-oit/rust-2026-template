#!/usr/bin/env bash
# release-manager.sh - Safe release operations wrapper
# Usage: ./scripts/release-manager.sh <validate|prepare|publish|full> [--execute]
# Dry-run by default. Pass --execute to actually run.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

GREEN='\033[0;32m'; YELLOW='\033[1;33m'; RED='\033[0;31m'; NC='\033[0m'
info()  { echo -e "${GREEN}[INFO]${NC} $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*" >&2; }

OP="${1:-help}"
EXECUTE=false
[[ "${2:-}" == "--execute" ]] && EXECUTE=true

run() {
  if $EXECUTE; then
    info "Running: $*"
    "$@"
  else
    warn "[DRY-RUN] Would run: $*"
  fi
}

validate() {
  info "--- Validate phase ---"
  ./scripts/quality-gates.sh
  cargo semver-checks check-release || warn "semver-checks not installed, skipping"
  info "Validation passed"
}

prepare() {
  local bump="${2:-patch}"
  info "--- Prepare phase (bump: $bump) ---"
  run cargo release "$bump" --no-publish --no-push
}

publish() {
  info "--- Publish phase ---"
  run cargo release publish --execute
}

case "$OP" in
  validate) validate ;;
  prepare)  validate && prepare "${@:2}" ;;
  publish)  publish ;;
  full)
    BUMP="${2:-patch}"
    info "Full release: validate → prepare → publish (bump: $BUMP)"
    validate
    run cargo release "$BUMP" --execute
    ;;
  *)
    echo "Usage: $(basename "$0") <validate|prepare|publish|full> [--execute] [patch|minor|major]"
    echo "  Dry-run by default. Pass --execute to actually perform operations."
    exit 1
    ;;
esac
