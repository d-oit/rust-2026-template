#!/usr/bin/env bash
# tests/maintenance_classification_test.sh
# Offline contract tests for the generated-maintenance fast path.
#
# Every case runs the real CLI entrypoints against fixture inputs, so the tests
# prove behaviour rather than grepping for strings. No case touches the network
# or requires credentials.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

CLASSIFIER="scripts/classify-maintenance-pr.py"
VALIDATOR="scripts/validate-dora-artifacts.py"
MAINTENANCE_VALIDATE="scripts/maintenance-validate.sh"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

PASS=0
FAIL=0

ok() {
  PASS=$((PASS + 1))
  echo "✓ $1"
}

bad() {
  FAIL=$((FAIL + 1))
  echo "✗ $1" >&2
}

# assert_exit <expected> <description> -- <command...>
assert_exit() {
  local expected="$1" description="$2"
  shift 3
  local actual=0
  "$@" >"$WORK/out.txt" 2>&1 || actual=$?
  if [[ "$actual" -eq "$expected" ]]; then
    ok "$description"
  else
    bad "$description (expected exit $expected, got $actual)"
    sed 's/^/    /' "$WORK/out.txt" >&2
  fi
}

# assert_contains <needle> <description> -- <command...>
assert_contains() {
  local needle="$1" description="$2"
  shift 3
  local actual=0
  "$@" >"$WORK/out.txt" 2>&1 || actual=$?
  if grep -qF -- "$needle" "$WORK/out.txt"; then
    ok "$description"
  else
    bad "$description (missing: $needle)"
    sed 's/^/    /' "$WORK/out.txt" >&2
  fi
}

dora_files='["reports/DORA-REPORT.md","reports/dora-history.jsonl","reports/dora-manifest.json"]'

classify() {
  # classify <files-json> <actor> <head-ref> <base-ref> [event]
  python3 "$CLASSIFIER" \
    --files-json "$1" \
    --actor "$2" \
    --head-ref "$3" \
    --base-ref "$4" \
    --event "${5:-pull_request}"
}

echo "=== Generated maintenance PR classification ==="

assert_contains '"maintenance_only": true' "DORA allowlist from the trusted bot qualifies" -- \
  classify "$dora_files" 'github-actions[bot]' 'automation/dora-report' 'main'

# Acceptance criterion for #503: the real file set of PR #494 must qualify.
assert_contains '"maintenance_only": true' "PR #494's exact file set qualifies for the fast path" -- \
  classify '["reports/DORA-REPORT.md","reports/dora-history.jsonl","reports/dora-manifest.json"]' \
  'github-actions[bot]' 'automation/dora-report' 'main'

assert_contains '"maintenance_only": false' "mixed DORA + Rust change does not qualify" -- \
  classify '["reports/DORA-REPORT.md","crates/sample-app/src/main.rs"]' \
  'github-actions[bot]' 'automation/dora-report' 'main'

assert_contains '"maintenance_only": false' "unknown path does not qualify" -- \
  classify '["reports/DORA-REPORT.md","reports/other-report.md"]' \
  'github-actions[bot]' 'automation/dora-report' 'main'

assert_contains '"maintenance_only": false' "non-allowlisted bot branch does not qualify" -- \
  classify "$dora_files" 'github-actions[bot]' 'automation/other' 'main'

assert_contains '"maintenance_only": false' "human actor does not qualify" -- \
  classify "$dora_files" 'd-oit' 'automation/dora-report' 'main'

assert_contains '"maintenance_only": false' "non-main base does not qualify" -- \
  classify "$dora_files" 'github-actions[bot]' 'automation/dora-report' 'develop'

assert_contains '"maintenance_only": false' "push event does not qualify" -- \
  classify "$dora_files" 'github-actions[bot]' 'automation/dora-report' 'main' 'push'

assert_contains 'complete changed-file list is unavailable' "missing file list fails closed" -- \
  python3 "$CLASSIFIER" --actor 'github-actions[bot]' --head-ref automation/dora-report \
  --base-ref main --event pull_request

assert_contains 'changed-file list is empty' "empty file list fails closed" -- \
  classify '[]' 'github-actions[bot]' 'automation/dora-report' 'main'

assert_contains '"maintenance_only": false' "malformed file list JSON fails closed" -- \
  classify '{"not":"a list"}' 'github-actions[bot]' 'automation/dora-report' 'main'

# A policy whose allowlist hides a wildcard must be rejected outright.
jq '.allowed_paths = ["reports/**"]' config/maintenance-pr.json >"$WORK/wildcard.json"
assert_exit 1 "wildcard allowlist is rejected" -- \
  python3 "$CLASSIFIER" --files-json "$dora_files" --actor 'github-actions[bot]' \
  --head-ref automation/dora-report --base-ref main --event pull_request \
  --policy "$WORK/wildcard.json"

jq '.allowed_paths = ["reports/../secrets.env"]' config/maintenance-pr.json >"$WORK/traversal.json"
assert_exit 1 "traversal allowlist is rejected" -- \
  python3 "$CLASSIFIER" --files-json "$dora_files" --actor 'github-actions[bot]' \
  --head-ref automation/dora-report --base-ref main --event pull_request \
  --policy "$WORK/traversal.json"

jq 'del(.trusted_actors)' config/maintenance-pr.json >"$WORK/incomplete.json"
assert_exit 1 "incomplete policy is rejected" -- \
  python3 "$CLASSIFIER" --files-json "$dora_files" --actor 'github-actions[bot]' \
  --head-ref automation/dora-report --base-ref main --event pull_request \
  --policy "$WORK/incomplete.json"

assert_exit 3 "--require-maintenance fails for an ordinary PR" -- \
  python3 "$CLASSIFIER" --files-json '["crates/sample-app/src/main.rs"]' \
  --actor 'github-actions[bot]' --head-ref automation/dora-report --base-ref main \
  --event pull_request --require-maintenance

assert_exit 0 "--require-maintenance passes for an allowlisted PR" -- \
  python3 "$CLASSIFIER" --files-json "$dora_files" --actor 'github-actions[bot]' \
  --head-ref automation/dora-report --base-ref main --event pull_request \
  --require-maintenance

assert_contains '"changed_files_digest": "sha256:' "classification records a stable file digest" -- \
  classify "$dora_files" 'github-actions[bot]' 'automation/dora-report' 'main'

echo ""
echo "=== DORA artifact validation (reuses the real generator's schema) ==="

assert_exit 0 "committed DORA artifacts satisfy the contract" -- python3 "$VALIDATOR"

jq '.metrics.change_lead_time.avg_hours = 999.5' reports/dora-manifest.json >"$WORK/drift.json"
assert_contains "does not match manifest" "report/manifest drift is rejected" -- \
  python3 "$VALIDATOR" --manifest "$WORK/drift.json"

jq 'del(.metrics.deployment_frequency)' reports/dora-manifest.json >"$WORK/nometric.json"
assert_contains "must be an object" "missing metric section is rejected" -- \
  python3 "$VALIDATOR" --manifest "$WORK/nometric.json"

jq '.schema_version = "2.0"' reports/dora-manifest.json >"$WORK/badschema.json"
assert_contains "schema_version" "unsupported manifest schema is rejected" -- \
  python3 "$VALIDATOR" --manifest "$WORK/badschema.json"

jq '.input_hashes.releases = "not-a-hash"' reports/dora-manifest.json >"$WORK/badhash.json"
assert_contains "SHA-256" "malformed input hash is rejected" -- \
  python3 "$VALIDATOR" --manifest "$WORK/badhash.json"

jq '.metrics.overall_tier = "Elite"' reports/dora-manifest.json >"$WORK/badtier.json"
assert_contains "disagrees with the per-metric tiers" "inconsistent overall tier is rejected" -- \
  python3 "$VALIDATOR" --manifest "$WORK/badtier.json"

{ head -1 reports/dora-history.jsonl; echo '{not json'; } >"$WORK/badhistory.jsonl"
assert_contains "is not valid JSON" "malformed history line is rejected" -- \
  python3 "$VALIDATOR" --history "$WORK/badhistory.jsonl"

head -1 reports/dora-history.jsonl >"$WORK/stale.jsonl"
assert_contains "does not match manifest now_iso" "stale history snapshot is rejected" -- \
  python3 "$VALIDATOR" --history "$WORK/stale.jsonl"

: >"$WORK/empty.jsonl"
assert_contains "history file is empty" "empty history file is rejected" -- \
  python3 "$VALIDATOR" --history "$WORK/empty.jsonl"

assert_contains "report is missing" "missing report is rejected" -- \
  python3 "$VALIDATOR" --report "$WORK/absent.md"

sed 's/\*\*Low\*\*/**Elite**/' reports/DORA-REPORT.md >"$WORK/tampered.md"
assert_contains "disagrees with the manifest" "tampered report metric row is rejected" -- \
  python3 "$VALIDATOR" --report "$WORK/tampered.md"

echo ""
echo "=== Maintenance validator dispatch ==="

echo '{"maintenance_only":true,"validators":["dora-artifacts"]}' >"$WORK/granted.json"
assert_exit 0 "granted classification runs the DORA validator" -- \
  bash "$MAINTENANCE_VALIDATE" --classification "$WORK/granted.json"

echo '{"maintenance_only":false,"validators":["dora-artifacts"]}' >"$WORK/denied.json"
assert_exit 1 "denied classification is refused" -- \
  bash "$MAINTENANCE_VALIDATE" --classification "$WORK/denied.json"

echo '{"maintenance_only":true,"validators":[]}' >"$WORK/novalidators.json"
assert_exit 1 "empty validator list fails closed" -- \
  bash "$MAINTENANCE_VALIDATE" --classification "$WORK/novalidators.json"

echo '{"maintenance_only":true,"validators":["unknown-validator"]}' >"$WORK/unknown.json"
assert_exit 1 "unknown validator fails closed" -- \
  bash "$MAINTENANCE_VALIDATE" --classification "$WORK/unknown.json"

assert_exit 1 "missing classification file fails closed" -- \
  bash "$MAINTENANCE_VALIDATE" --classification "$WORK/does-not-exist.json"

echo ""
echo "=== Classification policy schema ==="

python3 - <<'PY'
import json
import sys

try:
    import jsonschema
except ImportError:  # pragma: no cover - optional dependency
    print("SKIP: jsonschema is not installed; policy schema check skipped")
    sys.exit(0)

schema = json.load(open("schema/maintenance-pr.schema.json", encoding="utf-8"))
policy = json.load(open("config/maintenance-pr.json", encoding="utf-8"))
jsonschema.validate(policy, schema)

# The classifier's own stricter rules must also reject widening attempts.
for mutation in (
    {"allowed_paths": ["reports/**"]},
    {"allowed_paths": ["reports/../secrets.env"]},
    {"trusted_actors": []},
    {"schema_version": "2.0"},
):
    candidate = dict(policy)
    candidate.update(mutation)
    try:
        jsonschema.validate(candidate, schema)
    except jsonschema.ValidationError:
        continue
    print(f"FAIL: policy schema accepted unsafe mutation {mutation}", file=sys.stderr)
    sys.exit(1)

print("✓ maintenance policy satisfies its schema and rejects unsafe widening")
PY

echo ""
if [[ $FAIL -gt 0 ]]; then
  echo "FAIL: $FAIL assertion(s) failed, $PASS passed." >&2
  exit 1
fi
echo "PASS: all $PASS maintenance classification and DORA artifact assertions passed."
