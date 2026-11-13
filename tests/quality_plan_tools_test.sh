#!/usr/bin/env bash
# tests/quality_plan_tools_test.sh
# Keeps scripts/quality-plan-tools.py in sync with the Rust check names.
#
# The CI tool-installation step maps selected checks to tool groups. If a check
# is renamed in `crates/xtask/src/quality.rs` without updating the mapping, CI
# would install the wrong tools (or none) and the gate could fail for the wrong
# reason. This test derives the expected names from the Rust source, so drift
# breaks the build instead of silently degrading CI.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

python3 - <<'PY'
import re
import sys
from pathlib import Path

quality_rs = Path("crates/xtask/src/quality.rs").read_text(encoding="utf-8")
in_names = False
expected = []
for line in quality_rs.splitlines():
    if "pub const fn name(self) -> &'static str" in line:
        in_names = True
        continue
    if in_names:
        match = re.search(r'=>\s*"([^"]+)",\s*$', line)
        if match:
            expected.append(match.group(1))
        elif line.strip() == "}":
            break

if len(expected) < 20:
    print(f"FAIL: only parsed {len(expected)} check names from quality.rs", file=sys.stderr)
    sys.exit(1)

script = Path("scripts/quality-plan-tools.py").read_text(encoding="utf-8")
mapping_block = script.split("CHECK_GROUPS: dict[str, str] = {", 1)[1].split("}", 1)[0]
mapped = re.findall(r'^\s*"([^"]+)":', mapping_block, re.MULTILINE)

missing = [name for name in expected if name not in mapped]
extra = [name for name in mapped if name not in expected]

if missing:
    print(f"FAIL: checks missing from the tool-group mapping: {missing}", file=sys.stderr)
if extra:
    print(f"FAIL: tool-group mapping names unknown checks: {extra}", file=sys.stderr)
if missing or extra:
    sys.exit(1)

print(f"PASS: tool-group mapping covers all {len(expected)} quality checks")
PY

# The tarpaulin-style smoke checks below exercise the real CLI.
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

printf 'Planned Checks:\n  - Markdown Lint (markdownlint-cli2)\n  - LOC Limits\n' >"$WORK/md.txt"
groups=$(python3 scripts/quality-plan-tools.py "$WORK/md.txt")
[[ "$groups" == "markdown" ]] || {
  echo "FAIL: a Markdown-only plan must need only the markdown tool group (got '$groups')" >&2
  exit 1
}
echo "✓ Markdown-only plan installs only markdown tooling"

printf 'Planned Checks:\n  - Rust Security Audit\n  - Rust Dependency Policy (Deny)\n' >"$WORK/security.txt"
groups=$(python3 scripts/quality-plan-tools.py "$WORK/security.txt")
[[ "$groups" == "security" ]] || {
  echo "FAIL: a security-only plan must need only the security tool group (got '$groups')" >&2
  exit 1
}
echo "✓ Security-only plan installs only security tooling"

printf 'Planned Checks:\n' >"$WORK/empty.txt"
if python3 scripts/quality-plan-tools.py "$WORK/empty.txt" >/dev/null 2>&1; then
  echo "FAIL: an empty plan must fail closed" >&2
  exit 1
fi
echo "✓ Empty plan fails closed instead of installing nothing"

printf 'Planned Checks:\n  - Mystery Check\n' >"$WORK/unknown.txt"
if python3 scripts/quality-plan-tools.py "$WORK/unknown.txt" >/dev/null 2>&1; then
  echo "FAIL: an unmapped check must fail closed" >&2
  exit 1
fi
echo "✓ Unmapped check fails closed"

echo "PASS: quality plan tool-group resolution behaves correctly"
