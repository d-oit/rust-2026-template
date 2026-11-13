#!/usr/bin/env python3
"""Map an xtask quality plan to the tool groups CI must install.

`xtask quality plan` prints the resolved check selection; installing every tool
for every PR wastes minutes on Markdown-only changes. This maps selected checks
to coarse tool groups so the workflow can install only what it needs, and emits
``cargo`` when a Cargo toolchain is required.

The mapping is intentionally conservative: an unrecognised check name is an
error rather than an implicit "install nothing", because a silently ignored
check would run without its tool and the gate could pass for the wrong reason.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

# Check name -> tool group. Mirrors `crate::quality::QualityCheck::name`; keep
# the two in sync (tests/quality_plan_tools_test.sh asserts they agree).
CHECK_GROUPS: dict[str, str] = {
    "LOC Limits": "none",
    "Rust Format": "cargo",
    "Rust Clippy": "rust-build",
    "Rust Build": "rust-build",
    "Rust Tests": "cargo",
    "Rust Doc Tests": "cargo",
    "Rust Doc Build": "cargo",
    "Rust Security Audit": "security",
    "Rust Dependency Policy (Deny)": "security",
    "Rust Unused Dependencies (Machete)": "security",
    "Rust MSRV Audit": "rust-build",
    "Shell Script Lint (ShellCheck)": "shell",
    "Markdown Lint (markdownlint-cli2)": "markdown",
    "YAML Lint (yamllint)": "yaml",
    "Privacy Check (No emails)": "none",
    "Secret Scan": "none",
    "Skill Validation": "none",
    "Skill Evaluations": "none",
    "ADR Compliance": "none",
    "LLM Context Files Check": "none",
    "GitHub Actions Workflow Validation": "yaml",
    "GitHub Actions Semantic Check (actionlint)": "actionlint",
}

GROUP_ORDER = ("cargo", "rust-build", "security", "shell", "yaml", "markdown", "actionlint")


def parse_plan(text: str) -> list[str]:
    """Extract check names from `xtask quality plan` output."""
    checks: list[str] = []
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("- "):
            checks.append(stripped[2:].strip())
    return checks


def required_groups(checks: list[str]) -> list[str]:
    """Resolve the ordered, de-duplicated tool groups for `checks`."""
    groups: set[str] = set()
    unknown: list[str] = []
    for check in checks:
        group = CHECK_GROUPS.get(check)
        if group is None:
            unknown.append(check)
            continue
        groups.add(group)
    if unknown:
        raise ValueError(f"unmapped quality check(s): {', '.join(sorted(unknown))}")
    groups.discard("none")
    return [group for group in GROUP_ORDER if group in groups]


def main(argv: list[str] | None = None) -> int:
    """Print a comma-separated tool-group list for a plan file."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("plan", type=Path, help="Output file from `xtask quality plan`")
    args = parser.parse_args(argv)

    try:
        checks = parse_plan(args.plan.read_text(encoding="utf-8"))
        groups = required_groups(checks)
    except (OSError, ValueError) as error:
        print(f"::error::cannot resolve tool groups: {error}", file=sys.stderr)
        return 1

    if not checks:
        print("::error::quality plan is empty; refusing to install nothing", file=sys.stderr)
        return 1

    print(",".join(groups))
    return 0


if __name__ == "__main__":
    sys.exit(main())
