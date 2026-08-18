#!/usr/bin/env python3
"""Validates .agents/ci/quality-run.json against schema/ci-telemetry.schema.json."""

import json
import sys
from pathlib import Path


def check_no_sensitive_keys(obj, path=""):
    """Recursively checks that no object keys contain sensitive words (token, password, key, credential)."""
    if isinstance(obj, dict):
        for k, v in obj.items():
            k_lower = k.lower()
            if any(forbidden in k_lower for forbidden in ["token", "password", "private_key", "credential"]):
                print(f"Error: Telemetry artifact contains sensitive key '{k}' at path '{path}'", file=sys.stderr)
                sys.exit(1)
            check_no_sensitive_keys(v, f"{path}.{k}")
    elif isinstance(obj, list):
        for idx, item in enumerate(obj):
            check_no_sensitive_keys(item, f"{path}[{idx}]")


def main():
    schema_path = Path("schema/ci-telemetry.schema.json")
    artifact_path = Path(".agents/ci/quality-run.json")

    if not schema_path.exists():
        print(f"Error: Schema file not found at {schema_path}", file=sys.stderr)
        sys.exit(1)

    if not artifact_path.exists():
        print(f"Error: Telemetry artifact not found at {artifact_path}", file=sys.stderr)
        sys.exit(1)

    with open(schema_path, "r", encoding="utf-8") as f:
        schema = json.load(f)

    with open(artifact_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    # 1. Schema version check
    if data.get("schema_version") != schema["properties"]["schema_version"]["const"]:
        print(f"Error: Invalid schema_version: {data.get('schema_version')}", file=sys.stderr)
        sys.exit(1)

    # 2. Required top-level fields
    for req in schema["required"]:
        if req not in data:
            print(f"Error: Missing required top-level field '{req}'", file=sys.stderr)
            sys.exit(1)

    # 3. Scope validation
    scope = data["scope"]
    for req in schema["properties"]["scope"]["required"]:
        if req not in scope:
            print(f"Error: Missing required scope field '{req}'", file=sys.stderr)
            sys.exit(1)
    if scope["mode"] not in schema["properties"]["scope"]["properties"]["mode"]["enum"]:
        print(f"Error: Invalid scope mode '{scope['mode']}'", file=sys.stderr)
        sys.exit(1)

    # 4. Stages validation
    stages = data["stages"]
    if not isinstance(stages, list):
        print("Error: 'stages' must be a list", file=sys.stderr)
        sys.exit(1)

    stage_schema = schema["properties"]["stages"]["items"]
    for idx, stage in enumerate(stages):
        for req in stage_schema["required"]:
            if req not in stage:
                print(f"Error: Stage {idx} missing required field '{req}'", file=sys.stderr)
                sys.exit(1)
        if stage["status"] not in stage_schema["properties"]["status"]["enum"]:
            print(f"Error: Stage {idx} invalid status '{stage['status']}'", file=sys.stderr)
            sys.exit(1)
        if stage["cache"] not in stage_schema["properties"]["cache"]["enum"]:
            print(f"Error: Stage {idx} invalid cache state '{stage['cache']}'", file=sys.stderr)
            sys.exit(1)
        if not isinstance(stage["duration_ms"], int) or stage["duration_ms"] < 0:
            print(f"Error: Stage {idx} invalid duration_ms '{stage['duration_ms']}'", file=sys.stderr)
            sys.exit(1)

    # 5. Toolchain validation
    tc = data["toolchain"]
    for req in schema["properties"]["toolchain"]["required"]:
        if req not in tc:
            print(f"Error: Toolchain missing required field '{req}'", file=sys.stderr)
            sys.exit(1)

    # 6. Safety check: ensure no sensitive keys are leaked
    check_no_sensitive_keys(data)

    print("✓ CI Telemetry artifact schema validation OK!")


if __name__ == "__main__":
    main()
