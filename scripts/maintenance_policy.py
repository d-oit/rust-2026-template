#!/usr/bin/env python3
"""Shared policy engine for generated maintenance pull requests.

Both the CI classifier (``scripts/classify-maintenance-pr.py``) and the merge
policy gate (``scripts/pr-merge-policy.py``) import this module, so there is one
implementation of the allowlist, the identity checks, and the decision rules.

The module is deliberately import-only: it performs no I/O at import time and no
network access at all. Callers supply the file list, the actor/branch metadata,
and any check-run data, which keeps both tools deterministic and testable.

Every input is treated as untrusted. A missing file list, an unknown path, an
untrusted actor or branch, a non-``main`` base, a non-``pull_request`` event, or
a malformed policy file all resolve to ``maintenance_only=False`` (or raise
[`PolicyError`]) so the caller falls back to full CI. Nothing here inspects the
PR title, body, or labels: those are user-controlled and must never widen a fast
path.
"""

from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path
from typing import Any

# Default policy location, relative to the repository root.
POLICY_PATH = Path("config/maintenance-pr.json")

# A literal, repository-relative path. Globs, traversal segments, absolute
# prefixes, backslashes, and control characters are rejected outright: the
# allowlist must never be able to widen itself.
_SAFE_PATH = re.compile(r"^[A-Za-z0-9._/-]+$")


class PolicyError(ValueError):
    """The checked-in policy file is missing, malformed, or unsafe."""


def load_policy(path: Path = POLICY_PATH) -> dict[str, Any]:
    """Read and validate the maintenance policy, raising `PolicyError` on drift."""
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as error:
        raise PolicyError(f"maintenance policy not found: {path}") from error
    except json.JSONDecodeError as error:
        raise PolicyError(f"maintenance policy is not valid JSON: {error}") from error

    if not isinstance(raw, dict):
        raise PolicyError("maintenance policy must be a JSON object")

    required = (
        "schema_version",
        "base_ref",
        "trusted_actors",
        "trusted_branches",
        "allowed_paths",
        "validators",
        "quality_gate_check_name",
        "github_actions_app_id",
        "codacy",
    )
    missing = [key for key in required if key not in raw]
    if missing:
        raise PolicyError(f"maintenance policy is missing keys: {', '.join(missing)}")
    if raw["schema_version"] != "1.0":
        raise PolicyError(
            f"unsupported maintenance policy schema_version: {raw['schema_version']!r}"
        )

    for key in ("trusted_actors", "trusted_branches", "allowed_paths", "validators"):
        value = raw[key]
        if not isinstance(value, list) or not value:
            raise PolicyError(f"{key} must be a non-empty list")
        if not all(isinstance(entry, str) and entry for entry in value):
            raise PolicyError(f"{key} must contain only non-empty strings")

    allowed = raw["allowed_paths"]
    if len(set(allowed)) != len(allowed):
        raise PolicyError("allowed_paths contains duplicates")
    for entry in allowed:
        if not _SAFE_PATH.match(entry) or ".." in entry.split("/") or entry.startswith("/"):
            raise PolicyError(
                f"allowed_paths entry {entry!r} is not a literal repository-relative path"
            )
    if any("*" in entry or "?" in entry for entry in allowed):
        raise PolicyError(
            "allowed_paths must enumerate files literally; wildcards are not allowed"
        )

    codacy = raw["codacy"]
    if not isinstance(codacy, dict) or not codacy.get("check_name"):
        raise PolicyError("codacy.check_name must be a non-empty string")
    if not isinstance(codacy.get("app_id"), int):
        raise PolicyError("codacy.app_id must be an integer")
    gate_app = raw.get("github_actions_app_id")
    if not isinstance(gate_app, int) or gate_app <= 0:
        raise PolicyError("github_actions_app_id must be a positive integer")
    if not isinstance(raw.get("quality_gate_check_name"), str):
        raise PolicyError("quality_gate_check_name must be a non-empty string")

    return raw


def load_file_list(raw: str | None, path: Path | None) -> list[str] | None:
    """Load the complete changed-file list, or `None` when it is unavailable."""
    try:
        if path is not None:
            text = path.read_text(encoding="utf-8")
        elif raw is not None:
            text = raw
        else:
            return None
        payload = json.loads(text)
    except (OSError, json.JSONDecodeError):
        return None

    # Accept a bare array, the GitHub REST file objects, and the `--slurp`
    # paginated shape (a list of pages, each holding a list of file objects).
    if isinstance(payload, dict):
        payload = payload.get("files")
    if isinstance(payload, list) and payload and all(isinstance(page, list) for page in payload):
        payload = [entry for page in payload for entry in page]
    if not isinstance(payload, list):
        return None
    if not payload:
        # An explicitly empty file list is distinguishable from an unavailable
        # one: both fail closed, but the diagnostic should say which happened.
        return []

    files: list[str] = []
    for entry in payload:
        if isinstance(entry, str):
            files.append(entry)
        elif isinstance(entry, dict) and isinstance(entry.get("filename"), str):
            files.append(entry["filename"])
        elif isinstance(entry, dict) and isinstance(entry.get("path"), str):
            files.append(entry["path"])
        else:
            return None
    return files


def file_list_digest(files: list[str]) -> str:
    """Stable digest of the changed-file set, for CI telemetry and diagnostics."""
    joined = "\n".join(sorted(files))
    return "sha256:" + hashlib.sha256(joined.encode("utf-8")).hexdigest()


def classify(
    files: list[str] | None,
    *,
    actor: str | None,
    head_ref: str | None,
    base_ref: str | None,
    event: str | None,
    policy: dict[str, Any],
) -> dict[str, Any]:
    """Pure classification decision. Never raises for untrusted input."""
    decision: dict[str, Any] = {
        "maintenance_only": False,
        "reason": "",
        "changed_paths": sorted(files or []),
        "changed_files_digest": "",
        "validators": [],
        "schema_version": policy["schema_version"],
    }

    if files is None:
        decision["reason"] = "complete changed-file list is unavailable"
        return decision
    if not files:
        decision["reason"] = "changed-file list is empty"
        return decision

    decision["changed_files_digest"] = file_list_digest(files)

    if event != "pull_request":
        decision["reason"] = f"event {event!r} is not a pull_request"
        return decision
    if base_ref != policy["base_ref"]:
        decision["reason"] = f"base ref {base_ref!r} is not {policy['base_ref']!r}"
        return decision
    if actor not in policy["trusted_actors"]:
        decision["reason"] = f"actor {actor!r} is not a trusted automation identity"
        return decision
    if head_ref not in policy["trusted_branches"]:
        decision["reason"] = f"head ref {head_ref!r} is not a trusted automation branch"
        return decision

    unexpected = sorted(set(files) - set(policy["allowed_paths"]))
    if unexpected:
        decision["reason"] = "changed files outside the allowlist: " + ", ".join(unexpected)
        return decision

    decision["maintenance_only"] = True
    decision["validators"] = list(policy["validators"])
    decision["reason"] = "every changed file matches the reviewed maintenance allowlist"
    return decision
