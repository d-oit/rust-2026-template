#!/usr/bin/env bash
# tests/ci_telemetry/telemetry_integration_test.sh
# Integration test to verify quality gate CI telemetry contract & schema validation.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

cd "${REPO_ROOT}"

# CI runs this suite inside the quality-gate job, which has already executed the tier;
# `--use-existing` skips the re-run so the artifacts under test are the job's own.
JSON_ARTIFACT=".agents/ci/quality-run.json"
SUMMARY_ARTIFACT=".agents/ci/quality-summary.md"
SCHEMA="schema/ci-telemetry.schema.json"

TARGET_TIER="pull-request"
if [[ "${1:-}" == "--use-existing" ]]; then
    echo "==> Using telemetry artifacts produced by the current job"
    if [[ -f "${JSON_ARTIFACT}" ]]; then
        TARGET_TIER=$(python3 -c "import json; print(json.load(open('${JSON_ARTIFACT}')).get('tier', 'pull-request'))")
    fi
else
    TARGET_TIER="${1:-pull-request}"
    echo "==> Running xtask quality run --tier ${TARGET_TIER} to generate telemetry..."
    cargo run --quiet -p xtask --bin xtask -- quality run --tier "${TARGET_TIER}"
fi
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

assert data.get('schema_version') == 2, 'Invalid schema_version'
assert 'tier' in data and data['tier'], 'Missing tier'
assert 'plan_source' in data and data['plan_source'], 'Missing plan_source'
assert 'scope' in data and 'mode' in data['scope'], 'Missing scope'
assert isinstance(data.get('stages'), list), 'Missing stages list'
assert 'toolchain' in data, 'Missing toolchain'
assert 'rustc' in data['toolchain'], 'Missing rustc in toolchain'
assert 'cargo' in data['toolchain'], 'Missing cargo in toolchain'
assert 'nextest' in data['toolchain'], 'Missing nextest in toolchain'
assert 'fingerprint' in data, 'Missing fingerprint'
assert 'head_commit' in data['fingerprint'], 'Missing head_commit in fingerprint'
assert 'worktree_hash' in data['fingerprint'], 'Missing worktree_hash in fingerprint'
assert 'policy_hash' in data['fingerprint'], 'Missing policy_hash in fingerprint'

print('Structural verification PASSED')
"

echo "==> Verifying xtask quality status reports GREEN after run..."
cargo run --quiet -p xtask --bin xtask -- quality status --tier "${TARGET_TIER}"
echo "==> Telemetry integration test completed successfully!"
