#!/usr/bin/env bash
# scripts/quality-gates.sh
# Thin wrapper that delegates to cargo xtask quality run.

set -uo pipefail

# Find repository root
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT" || exit 1

# Forward arguments to cargo xtask quality run
exec cargo run --bin xtask -- quality run "$@"
