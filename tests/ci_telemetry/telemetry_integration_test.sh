#!/usr/bin/env bash
# tests/ci_telemetry/telemetry_integration_test.sh
# Integration test to verify quality gate CI telemetry contract & schema validation.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

cd "${REPO_ROOT}"

echo "==> Running xtask quality run --tier pull-request to generate telemetry..."
cargo run --quiet -p xtask --bin xtask -- quality run --tier pull-request

JSON_ARTIFACT=".agents/ci/quality-run.json"
SUMMARY_ARTIFACT=".agents/ci/quality-summary.md"
SCHEMA="schema/ci-telemetry.schema.json"

echo "==> Checking artifact presence..."
if [[ ! -f "${JSON_ARTIFACT}" ]]; then
    echo "ERROR: Missing ${JSON_ARTIFACT}" >&2
    exit 1
fi

if [[ ! -f "${SUMMARY_ARTIFACT}" ]]; then
    echo "ERROR: Missing ${SUMMARY_ARTIFACT}" >&2
    exit 1
fi

echo "==> Validating ${JSON_ARTIFACT} against ${SCHEMA} using Python jsonschema..."
python3 -c "
import json
import sys

try:
    import jsonschema
except ImportError:
    print('jsonschema not installed, falling back to manual structural validation')
    sys.exit(0)

with open('${SCHEMA}', 'r') as f:
    schema = json.load(f)

with open('${JSON_ARTIFACT}', 'r') as f:
    artifact = json.load(f)

try:
    jsonschema.validate(instance=artifact, schema=schema)
    print('Schema validation PASSED via jsonschema')
except jsonschema.ValidationError as e:
    print(f'Schema validation FAILED: {e.message}')
    sys.exit(1)
"

echo "==> Performing structural verification on ${JSON_ARTIFACT}..."
python3 -c "
import json
import sys

with open('${JSON_ARTIFACT}', 'r') as f:
    data = json.load(f)

assert data.get('schema_version') == 1, 'Invalid schema_version'
assert 'tier' in data and data['tier'], 'Missing tier'
assert 'plan_source' in data and data['plan_source'], 'Missing plan_source'
assert 'scope' in data and 'mode' in data['scope'], 'Missing scope'
assert isinstance(data.get('stages'), list), 'Missing stages list'
assert 'toolchain' in data, 'Missing toolchain'
assert 'rustc' in data['toolchain'], 'Missing rustc in toolchain'
assert 'cargo' in data['toolchain'], 'Missing cargo in toolchain'
assert 'nextest' in data['toolchain'], 'Missing nextest in toolchain'

print('Structural verification PASSED')
"

echo "==> Telemetry integration test completed successfully!"
