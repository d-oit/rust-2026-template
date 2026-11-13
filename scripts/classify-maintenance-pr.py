#!/usr/bin/env python3
"""Fail-closed classifier for generated maintenance pull requests.

The decision logic lives in ``scripts/maintenance_policy.py``, which the merge
policy gate imports too. This module is the CI entry point: it reads the
complete changed-file list, prints the decision as JSON, and optionally turns a
negative classification into a non-zero exit.

Nothing here inspects the PR title, body, or labels: those are user-controlled
and must never widen a fast path.

Usage::

    python3 scripts/classify-maintenance-pr.py \\
        --files-json /tmp/pr-files.json \\
        --actor github-actions[bot] \\
        --head-ref automation/dora-report \\
        --base-ref main \\
        --event pull_request

Writes a JSON decision to stdout. ``--require-maintenance`` turns a negative
classification into exit code 3 so a workflow can refuse to continue.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import maintenance_policy  # noqa: E402  - resolved from this script's directory

POLICY_PATH = maintenance_policy.POLICY_PATH

EXIT_OK = 0
EXIT_USAGE = 1
EXIT_NOT_MAINTENANCE = 3


def main(argv: list[str] | None = None) -> int:
    """Entry point for the CI classifier."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--files-json",
        help="Complete changed-file list as JSON (array of paths or GitHub file objects)",
    )
    parser.add_argument(
        "--files-from",
        type=Path,
        help="Path to a file holding the complete changed-file list as JSON",
    )
    parser.add_argument("--actor", help="Login of the PR author, e.g. github-actions[bot]")
    parser.add_argument("--head-ref", help="PR head branch name")
    parser.add_argument("--base-ref", help="PR base branch name")
    parser.add_argument("--event", help="GitHub event name, e.g. pull_request")
    parser.add_argument(
        "--policy",
        type=Path,
        default=POLICY_PATH,
        help="Path to the maintenance classification policy",
    )
    parser.add_argument(
        "--require-maintenance",
        action="store_true",
        help="Exit non-zero (3) unless the PR qualifies for the fast path",
    )
    args = parser.parse_args(argv)

    try:
        policy = maintenance_policy.load_policy(args.policy)
    except maintenance_policy.PolicyError as error:
        print(f"::error::maintenance classification policy rejected: {error}")
        print(json.dumps({"maintenance_only": False, "reason": str(error)}, indent=2))
        return EXIT_USAGE

    files = maintenance_policy.load_file_list(args.files_json, args.files_from)
    decision = maintenance_policy.classify(
        files,
        actor=args.actor,
        head_ref=args.head_ref,
        base_ref=args.base_ref,
        event=args.event,
        policy=policy,
    )
    print(json.dumps(decision, indent=2, sort_keys=True))

    if args.require_maintenance and not decision["maintenance_only"]:
        return EXIT_NOT_MAINTENANCE
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
