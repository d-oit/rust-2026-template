#!/usr/bin/env bash
# scripts/check-template-version.sh
# Guards against template version drift between the changelog and the README badge.
#
# The template release workflow (.mimocode/commands/update-template-changelog.md) updates
# both; scoping a release to the changelog alone previously left the badge and the generated
# context files advertising the previous version.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

CHANGELOG=".template/CHANGELOG-TEMPLATE.md"
README="README.md"

latest="$(grep -m1 -E '^## \[[0-9]+\.[0-9]+\.[0-9]+\]' "${CHANGELOG}" \
  | sed -E 's/^## \[([0-9]+\.[0-9]+\.[0-9]+)\].*/\1/')"
badge="$(grep -m1 -oE 'badge/version-[0-9]+\.[0-9]+\.[0-9]+-blue' "${README}" \
  | sed -E 's|badge/version-([0-9]+\.[0-9]+\.[0-9]+)-blue|\1|')"

if [[ -z "${latest}" ]]; then
  echo "ERROR: no released version found in ${CHANGELOG}" >&2
  exit 1
fi

if [[ -z "${badge}" ]]; then
  echo "ERROR: no template version badge found in ${README}" >&2
  exit 1
fi

if [[ "${latest}" != "${badge}" ]]; then
  echo "ERROR: ${README} badge says ${badge} but ${CHANGELOG} documents ${latest}" >&2
  echo "Update the badge and anchor, then regenerate the context files:" >&2
  echo "  bash scripts/generate-llms-txt.sh" >&2
  exit 1
fi

echo "[OK] Template version ${latest} matches the README badge"
