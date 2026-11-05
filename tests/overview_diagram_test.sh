#!/usr/bin/env bash
# tests/overview_diagram_test.sh
# Guards the generated overview infographic against silently going stale.
#
# The committed diagram once advertised "12 Crates", "25 AI Skills", "45
# Pipeline Jobs" and "1 Quality Checks" while the repository had 15 workspace
# members, 26 skills, 40 CI jobs and 19 quality checks. Two causes:
#   * `_count_quality_checks` scraped `scripts/quality-gates.sh` for `cargo`
#     invocations, but that script became a thin wrapper around `cargo xtask`,
#     so the count collapsed to 1.
#   * nothing regenerated or verified the asset when the repo changed shape.
#
# This test compares the numbers embedded in the committed Excalidraw source
# against freshly discovered numbers, so drift fails CI instead of shipping.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

python3 - <<'PY'
import json
import sys
from pathlib import Path

sys.path.insert(0, ".agents/skills/architecture-diagram/scripts")

import generate_overview as gen

root = Path(".")
diagram = Path(".template/overview.excalidraw")

if not diagram.exists():
    sys.exit("FAIL: .template/overview.excalidraw is missing")

data = json.loads(diagram.read_text(encoding="utf-8"))
texts = [
    element["text"]
    for element in data.get("elements", [])
    if element.get("type") == "text" and not element.get("isDeleted")
]

expected = {
    "Workspace Members": len(gen.discover_crates(root)),
    "AI Skills": len(gen.discover_skills(root)),
    "CI Jobs": gen._count_pipeline_stages(root),
    "Quality Checks": gen._count_quality_checks(root),
}

failures = []
for label, value in expected.items():
    # The stat card renders the number immediately before its label.
    if label not in texts:
        failures.append(f"stat label {label!r} missing from the diagram")
        continue
    index = texts.index(label)
    rendered = texts[index - 1].strip() if index > 0 else "<none>"
    if rendered != str(value):
        failures.append(
            f"{label}: diagram says {rendered!r}, repository has {value}"
        )

# A zero or one here always means the counter regressed rather than that the
# project genuinely has one check.
if expected["Quality Checks"] <= 1:
    failures.append(
        f"Quality Checks resolved to {expected['Quality Checks']}; "
        "the counter is broken, not the repository"
    )

if failures:
    print("FAIL: overview diagram is out of date", file=sys.stderr)
    for failure in failures:
        print(f"  - {failure}", file=sys.stderr)
    print(
        "\nRegenerate with: bash scripts/sync-architecture.sh",
        file=sys.stderr,
    )
    sys.exit(1)

print("PASS: overview diagram stats match the repository")
for label, value in expected.items():
    print(f"  {label}: {value}")
PY
