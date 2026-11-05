#!/usr/bin/env bash
# scripts/ci-required-evaluate.sh
# Deterministic evaluator for required CI jobs in .github/workflows/ci.yml
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
python3 "${SCRIPT_DIR}/ci_required_evaluate.py" "$@"
