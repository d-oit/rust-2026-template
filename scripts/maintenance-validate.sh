#!/usr/bin/env bash
# scripts/maintenance-validate.sh
# Validators for the generated-maintenance fast path.
#
# A maintenance-only PR replaces the full pull-request quality tier with the
# validators named by config/maintenance-pr.json. This script runs them and
# fails closed: an unknown or empty validator list is an error, not a no-op, so
# a policy drift can never silently validate nothing.
#
# Usage:
#   ./scripts/maintenance-validate.sh --classification classification.json
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT" || exit 1

CLASSIFICATION=""
while [[ $# -gt 0 ]]; do
  case "$1" in
  --classification)
    CLASSIFICATION="${2:-}"
    shift 2
    ;;
  *)
    echo "Unknown argument: $1" >&2
    exit 1
    ;;
  esac
done

if [[ -z "$CLASSIFICATION" ]]; then
  echo "ERROR: --classification <path> is required." >&2
  exit 1
fi
if [[ ! -f "$CLASSIFICATION" ]]; then
  echo "ERROR: classification file not found: $CLASSIFICATION" >&2
  exit 1
fi

VALIDATORS=$(python3 - "$CLASSIFICATION" <<'PY'
import json, sys

path = sys.argv[1]
try:
    with open(path, encoding="utf-8") as handle:
        decision = json.load(handle)
except (OSError, json.JSONDecodeError) as error:
    print(f"ERROR: cannot read classification: {error}", file=sys.stderr)
    sys.exit(1)

if not decision.get("maintenance_only"):
    print("ERROR: classification does not grant the maintenance fast path.", file=sys.stderr)
    sys.exit(1)

validators = decision.get("validators")
if not isinstance(validators, list) or not validators:
    print("ERROR: classification names no validators (fail closed).", file=sys.stderr)
    sys.exit(1)

print(" ".join(validators))
PY
)

FAILED=0
for validator in $VALIDATORS; do
  case "$validator" in
  dora-artifacts)
    echo "--- Running maintenance validator: DORA artifacts ---"
    if ! python3 scripts/validate-dora-artifacts.py; then
      FAILED=1
    fi
    ;;
  *)
    echo "ERROR: unknown maintenance validator '${validator}' (fail closed)." >&2
    FAILED=1
    ;;
  esac
done

if [[ $FAILED -ne 0 ]]; then
  echo "::error::Generated maintenance validation failed."
  exit 1
fi

echo "PASS: generated maintenance validators succeeded for: ${VALIDATORS}"
