#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
FIXTURE_DIR="${SCRIPT_DIR}/fixtures/fuzz-deps-stale"
LOCKFILE="${FIXTURE_DIR}/Cargo.lock"
CHECK_SCRIPT="${REPO_ROOT}/scripts/check-fuzz-deps.sh"

if ! command -v cargo >/dev/null 2>&1; then
  echo "[SKIP] Cargo is required to validate the stale-lock fixture."
  exit 0
fi

missing_output="$(
  FUZZ_MANIFEST_PATH="${SCRIPT_DIR}/fixtures/fuzz-deps-missing/Cargo.toml" \
    "$CHECK_SCRIPT" 2>&1 || true
)"
if ! grep -q "Fuzz lockfile is missing or empty" <<<"$missing_output"; then
  echo "ERROR: a missing fuzz lockfile did not fail with an actionable message" >&2
  echo "$missing_output" >&2
  exit 1
fi

metadata_output="$(
  CARGO_NET_OFFLINE=true cargo metadata --locked \
    --manifest-path "${FIXTURE_DIR}/Cargo.toml" --format-version 1 2>&1 || true
)"
if ! grep -qE "lock file needs to be updated|--locked was passed" <<<"$metadata_output"; then
  echo "ERROR: stale fixture did not fail because its lockfile needs an update" >&2
  echo "$metadata_output" >&2
  exit 1
fi

before="$(sha256sum "$LOCKFILE")"
output="$(
  CARGO_NET_OFFLINE=true FUZZ_MANIFEST_PATH="${FIXTURE_DIR}/Cargo.toml" \
    "$CHECK_SCRIPT" 2>&1 || true
)"
after="$(sha256sum "$LOCKFILE")"

if [[ "$before" != "$after" ]]; then
  echo "ERROR: fuzz dependency check modified the stale fixture lockfile" >&2
  exit 1
fi
if ! grep -q "Fuzz Cargo.lock is stale" <<<"$output"; then
  echo "ERROR: stale fixture did not produce an actionable lockfile failure" >&2
  echo "$output" >&2
  exit 1
fi

echo "[PASS] Missing and stale fuzz lockfiles fail safely."
