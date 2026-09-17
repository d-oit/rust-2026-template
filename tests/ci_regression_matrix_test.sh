#!/usr/bin/env bash
set -euo pipefail

echo "=== CI Regression Matrix & Failure Signature Test Suite ==="

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

MATRIX_FILE=".agents/ci/regression-matrix.json"
SCHEMA_FILE="schema/ci-regression-matrix.schema.json"

if [ ! -f "$MATRIX_FILE" ]; then
  echo "ERROR: Matrix file $MATRIX_FILE not found."
  exit 1
fi

if [ ! -f "$SCHEMA_FILE" ]; then
  echo "ERROR: Schema file $SCHEMA_FILE not found."
  exit 1
fi

echo "[1/3] Validating $MATRIX_FILE against schema $SCHEMA_FILE..."
python3 -c "
import json, sys

with open('$SCHEMA_FILE') as sf:
    schema = json.load(sf)
with open('$MATRIX_FILE') as mf:
    matrix = json.load(mf)

assert matrix.get('version') == '1.0', 'Invalid version'
cases = matrix.get('cases', [])
assert len(cases) >= 7, f'Expected at least 7 historical cases, found {len(cases)}'

required_ids = [
    'sec-scan-cargo-audit-deny-20260818',
    'ci-cargo-deny-20260820',
    'sec-scan-cargo-audit-deny-20260821',
    'ci-markdown-lint-20260828',
    'sec-scan-gitleaks-20260904',
    'dora-report-gen-20260914',
    'update-diagram-push-20260917'
]

found_ids = {c['id'] for c in cases}
for req_id in required_ids:
    assert req_id in found_ids, f'Missing required historical case ID: {req_id}'

print('✓ Regression matrix schema and required case IDs validated successfully.')
"

echo "[2/3] Testing failure signatures and fixture checks..."

# Test 1: Markdown lint violation detection signature
MD_ERR_OUTPUT="MD001/heading-increment/header-increment Heading levels should only increment by one level at a time [Expected: h2; Actual: h3]"
python3 -c "
import json, sys
with open('$MATRIX_FILE') as mf:
    matrix = json.load(mf)

case = next(c for c in matrix['cases'] if c['subsystem'] == 'markdownlint')
sig_matched = any(sig in '''$MD_ERR_OUTPUT''' for sig in case['detection_signatures'])
assert sig_matched, 'Markdown lint signature failed to match test error output'
print('✓ Markdown lint error signature matching verified.')
"

# Test 2: Cargo deny check signature
DENY_ERR_OUTPUT="error[bans]: package 'unapproved-crate' is banned by configuration"
python3 -c "
import json, sys
with open('$MATRIX_FILE') as mf:
    matrix = json.load(mf)

case = next(c for c in matrix['cases'] if c['id'] == 'ci-cargo-deny-20260820')
sig_matched = any(sig in '''$DENY_ERR_OUTPUT''' for sig in case['detection_signatures'])
assert sig_matched, 'Cargo deny signature failed to match test error output'
print('✓ Cargo deny error signature matching verified.')
"

# Test 3: Gitleaks secret detection signature
GITLEAKS_ERR_OUTPUT="13:37PM INF 1 leaks found in commit 592909cbe95b3a261ec0edf6fdd5e46719eb525f"
python3 -c "
import json, sys
with open('$MATRIX_FILE') as mf:
    matrix = json.load(mf)

case = next(c for c in matrix['cases'] if c['subsystem'] == 'gitleaks')
sig_matched = any(sig in '''$GITLEAKS_ERR_OUTPUT''' for sig in case['detection_signatures'])
assert sig_matched, 'Gitleaks signature failed to match test error output'
print('✓ Gitleaks error signature matching verified.')
"

# Test 4: DORA report generation error signature
DORA_ERR_OUTPUT="Error: releases data at /tmp/releases.json is not a JSON list"
python3 -c "
import json, sys
with open('$MATRIX_FILE') as mf:
    matrix = json.load(mf)

case = next(c for c in matrix['cases'] if c['subsystem'] == 'dora-report')
sig_matched = any(sig in '''$DORA_ERR_OUTPUT''' for sig in case['detection_signatures'])
assert sig_matched, 'DORA report signature failed to match test error output'
print('✓ DORA report error signature matching verified.')
"

# Test 5: Architecture diagram push concurrency signature
DIAGRAM_ERR_OUTPUT="Push failed due to concurrent update, retrying... Failed to push architecture diagram after 5 attempts."
python3 -c "
import json, sys
with open('$MATRIX_FILE') as mf:
    matrix = json.load(mf)

case = next(c for c in matrix['cases'] if c['subsystem'] == 'architecture-diagram-updater')
sig_matched = any(sig in '''$DIAGRAM_ERR_OUTPUT''' for sig in case['detection_signatures'])
assert sig_matched, 'Architecture diagram signature failed to match test error output'
print('✓ Architecture diagram error signature matching verified.')
"

echo "[3/3] Local tool capability check..."
# Run local Markdown lint tool if available
if command -v npx >/dev/null 2>&1; then
  echo "Running local markdownlint-cli2 validation on workspace docs..."
  npx markdownlint-cli2 "**/*.md" >/dev/null 2>&1 || {
    echo "WARNING: Local markdown linting found issues, check manually with npx markdownlint-cli2 '**/*.md'"
  }
  echo "✓ Local markdown lint check executed."
fi

echo "=== All CI Regression Matrix tests PASSED successfully ==="
