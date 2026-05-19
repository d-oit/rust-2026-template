#!/usr/bin/env bash
# scripts/pre-release-hook.sh - Pre-release quality gate hook
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

echo "--- Running Pre-release Quality Gates ---"
bash scripts/quality-gates.sh
echo "--- Quality Gates Passed ---"
