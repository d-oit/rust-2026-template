#!/usr/bin/env bash
# tests/quality_gate_test.sh
# Simple integration test to ensure quality gates script runs correctly

set -euo pipefail

echo "Running quality gates test..."

# Run quality gates with check only
if bash ./scripts/quality-gates.sh; then
    echo "Quality gates passed!"
    exit 0
else
    echo "Quality gates failed!"
    exit 1
fi
