#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

FUZZ_MANIFEST="${FUZZ_MANIFEST_PATH:-${REPO_ROOT}/fuzz/Cargo.toml}"
if [[ ! -f "$FUZZ_MANIFEST" ]]; then
  echo "Fuzz manifest not found: $FUZZ_MANIFEST" >&2
  exit 1
fi

FUZZ_DIR="$(cd "$(dirname "$FUZZ_MANIFEST")" && pwd)"
FUZZ_LOCKFILE="${FUZZ_DIR}/Cargo.lock"
if [[ ! -s "$FUZZ_LOCKFILE" ]]; then
  echo "Fuzz lockfile is missing or empty: $FUZZ_LOCKFILE" >&2
  echo "Create it with 'cargo generate-lockfile --manifest-path \"$FUZZ_MANIFEST\"' and commit it." >&2
  exit 1
fi

if ! cargo metadata --locked --manifest-path "$FUZZ_MANIFEST" --format-version 1 >/dev/null; then
  echo "Fuzz Cargo.lock is stale for $FUZZ_MANIFEST." >&2
  echo "Update it intentionally with 'cargo generate-lockfile --manifest-path \"$FUZZ_MANIFEST\"', review the diff, and commit it." >&2
  exit 1
fi

cargo audit --file "$FUZZ_LOCKFILE"
cargo deny --manifest-path "$FUZZ_MANIFEST" --config "$REPO_ROOT/deny.toml" check
