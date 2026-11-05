#!/usr/bin/env bash
# tests/workflow_permissions_test.sh
# Guards the `permissions:` blocks of every workflow.
#
# Declaring any permission in a job sets every unlisted permission to `none`.
# `release.yml`'s `post-release-check` job declared only `issues: write` while
# also running `actions/checkout`, which needs `contents: read`, so the
# checkout step could not authenticate. The same failure mode previously took
# the whole Release workflow down at parse time when a `permissions:` block
# accidentally absorbed the job `outputs:` keys.
#
# Both classes are caught structurally here rather than by waiting for a
# release tag to exercise them.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

# `WORKFLOWS_DIR` allows the test to be pointed at a fixture tree.
WORKFLOWS_DIR="${WORKFLOWS_DIR:-.github/workflows}"

WORKFLOWS_DIR="${WORKFLOWS_DIR}" python3 - <<'PY'
import os
import sys
from pathlib import Path

import yaml

workflows_dir = Path(os.environ["WORKFLOWS_DIR"])

VALID_SCOPES = {
    "actions", "artifact-metadata", "attestations", "checks", "contents",
    "deployments", "discussions", "id-token", "issues", "models", "packages",
    "pages", "pull-requests", "repository-projects", "security-events",
    "statuses",
}

failures = []

for path in sorted(workflows_dir.glob("*.yml")):
    try:
        workflow = yaml.safe_load(path.read_text(encoding="utf-8"))
    except yaml.YAMLError as error:
        failures.append(f"{path.name}: unparseable YAML ({error})")
        continue

    for job_name, job in (workflow.get("jobs") or {}).items():
        if not isinstance(job, dict):
            continue

        permissions = job.get("permissions")
        if not isinstance(permissions, dict):
            continue

        # An unknown scope means `outputs:` (or another key) leaked into the
        # permissions block. GitHub rejects the workflow at parse time with
        # zero jobs, which is nearly invisible in the UI.
        unknown = set(permissions) - VALID_SCOPES - {"read-all", "write-all"}
        for scope in sorted(unknown):
            failures.append(
                f"{path.name}:{job_name}: unknown permission scope {scope!r} "
                "(did an `outputs:` block land under `permissions:`?)"
            )

        uses_checkout = any(
            "actions/checkout" in str(step.get("uses", ""))
            for step in (job.get("steps") or [])
        )
        if uses_checkout and "contents" not in permissions:
            failures.append(
                f"{path.name}:{job_name}: runs actions/checkout but does not "
                f"grant `contents` (permissions: {sorted(permissions)})"
            )

if failures:
    print("FAIL: workflow permission problems", file=sys.stderr)
    for failure in failures:
        print(f"  - {failure}", file=sys.stderr)
    sys.exit(1)

print("PASS: workflow permissions are complete and use only known scopes")
PY
