#!/usr/bin/env bash
# code-quality.sh - Rust code quality operations
# Usage: ./scripts/code-quality.sh <operation> [options]
# Operations: fmt | clippy | audit | check | fix
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

GREEN='\033[0;32m'; YELLOW='\033[1;33m'; RED='\033[0;31m'; NC='\033[0m'
info()  { echo -e "${GREEN}[INFO]${NC} $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*" >&2; }

OP="${1:-help}"

case "$OP" in
  fmt)
    info "Formatting workspace..."
    cargo fmt --all
    info "Format check..."
    cargo fmt --all -- --check
    ;;
  clippy)
    info "Running Clippy (CI parity: --workspace --tests)..."
    cargo clippy --workspace --tests -- -D warnings
    ;;
  audit)
    info "Running security audit..."
    cargo audit
    ;;
  check)
    info "Running full CI parity check..."
    cargo fmt --all -- --check
    cargo clippy --workspace --tests -- -D warnings
    cargo build --workspace
    cargo nextest run --all
    cargo test --doc
    cargo audit
    info "All checks passed!"
    ;;
  fix)
    info "Auto-fixing common issues..."
    cargo fmt --all
    cargo clippy --workspace --fix --allow-staged
    ;;
  *)
    echo "Usage: $(basename "$0") <fmt|clippy|audit|check|fix>"
    exit 1
    ;;
esac
