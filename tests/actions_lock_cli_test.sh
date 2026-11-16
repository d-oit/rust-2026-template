#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

cp -a "$REPO_ROOT/.github" "$TMP_DIR/"
cd "$TMP_DIR"
export GH_ACTIONS_LOCK_REPO_ROOT="$TMP_DIR"

gh actions-lock --verify-local

rm .github/workflows/actions.lock
if gh actions-lock --verify-local >/dev/null 2>&1; then
  echo "ERROR: missing lockfile was not detected" >&2
  exit 1
fi

cp "$REPO_ROOT/.github/workflows/actions.lock" .github/workflows/actions.lock
python3 - <<'PY'
import re
import sys
from pathlib import Path

path = Path(".github/workflows/ci.yml")
source = path.read_text(encoding="utf-8")
updated, count = re.subn(
    r"(uses:\s*actions/checkout@)[a-f0-9]{40}",
    lambda match: match.group(1) + "0" * 40,
    source,
    count=1,
)
if count != 1:
    sys.exit("ERROR: could not create the stale-pin fixture")
path.write_text(updated, encoding="utf-8")
PY

if gh actions-lock --verify-local >/dev/null 2>&1; then
  echo "ERROR: stale workflow dependency was not detected" >&2
  exit 1
fi

echo "PASS: missing and stale Actions lock entries are rejected"
