#!/usr/bin/env bash
# tests/ci_telemetry/test_telemetry.sh
# Integration test to verify CI telemetry emission, schema compliance, and summary generation.

set -euo pipefail

echo "=========================================================="
echo "Running CI Telemetry Integration Test"
echo "=========================================================="

# Execute xtask quality run
cargo run --quiet -p xtask --bin xtask -- quality run --tier pull-request

# Validate JSON artifact schema
python3 tests/ci_telemetry/validate_schema.py

# Verify summary markdown exists
SUMMARY_PATH=".agents/ci/quality-summary.md"
if [ ! -f "$SUMMARY_PATH" ]; then
    echo "Error: Telemetry summary markdown not found at $SUMMARY_PATH" >&2
    exit 1
fi

# Verify required sections in summary markdown
grep -q "Quality Run Telemetry (schema v1)" "$SUMMARY_PATH" || { echo "Missing title in $SUMMARY_PATH" >&2; exit 1; }
grep -q "Tier:" "$SUMMARY_PATH" || { echo "Missing Tier in $SUMMARY_PATH" >&2; exit 1; }
grep -q "Plan source:" "$SUMMARY_PATH" || { echo "Missing Plan source in $SUMMARY_PATH" >&2; exit 1; }
grep -q "Toolchain:" "$SUMMARY_PATH" || { echo "Missing Toolchain in $SUMMARY_PATH" >&2; exit 1; }

echo "✓ CI Telemetry integration test passed!"
