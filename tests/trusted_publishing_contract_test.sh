#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

python3 - <<'PY'
import json
import re
import subprocess
from pathlib import Path

import yaml

workflow_path = Path(".github/workflows/release.yml")
config_path = Path("config/trusted-publishing.json")
workflow = yaml.safe_load(workflow_path.read_text(encoding="utf-8"))
config = json.loads(config_path.read_text(encoding="utf-8"))

assert config["workflow_filename"] == workflow_path.name
assert config["environment"] == "release"
packages = config["packages"]
assert isinstance(packages, list)
assert len(packages) == len(set(packages)), "publish allowlist contains duplicates"

metadata = json.loads(
    subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
)
workspace = {package["name"]: package for package in metadata["packages"]}
for name in packages:
    assert name in workspace, f"allowlisted package does not exist: {name}"
    package = workspace[name]
    assert package["publish"] is None or "crates-io" in package["publish"], (
        f"allowlisted package is not publishable to crates.io: {name}"
    )
    assert package["version"] != "0.0.0", f"placeholder version is not publishable: {name}"

jobs = workflow["jobs"]
assert "preflight" in jobs and "verify" in jobs
assert "publish_plan" in jobs and "publish_crates" in jobs
plan = jobs["publish_plan"]
assert "verify" in plan["needs"]
assert {"has_packages", "packages"} <= set(plan["outputs"])

publish = jobs["publish_crates"]
assert {"host", "publish_plan"} <= set(publish["needs"])
assert "needs.publish_plan.outputs.has_packages == 'true'" in publish["if"]
assert publish["environment"] == config["environment"]
assert publish["permissions"] == {"contents": "read", "id-token": "write"}

steps = publish["steps"]
auth_steps = [step for step in steps if "crates-io-auth-action" in step.get("uses", "")]
assert len(auth_steps) == 1
assert auth_steps[0].get("id") == "auth"
assert re.fullmatch(r"rust-lang/crates-io-auth-action@[0-9a-f]{40}", auth_steps[0]["uses"])

publish_steps = [step for step in steps if "CARGO_REGISTRY_TOKEN" in step.get("env", {})]
assert len(publish_steps) == 1
assert publish_steps[0]["env"]["CARGO_REGISTRY_TOKEN"] == "${{ steps.auth.outputs.token }}"
assert "release-publish" in publish_steps[0]["run"]
assert "--workspace" not in publish_steps[0]["run"]
assert '"--locked"' in Path("scripts/trustpub.py").read_text(encoding="utf-8")

workflow_text = workflow_path.read_text(encoding="utf-8")
assert "secrets.CARGO_REGISTRY_TOKEN" not in workflow_text
for name, job in jobs.items():
    if name not in {"host", "publish_crates"}:
        assert (job.get("permissions") or {}).get("id-token") != "write", (
            f"unrelated job received id-token permission: {name}"
        )

readme = Path("README.md").read_text(encoding="utf-8")
summary = Path("docs/src/SUMMARY.md").read_text(encoding="utf-8")
guide_path = Path("docs/src/releasing/crates-io-trusted-publishing.md")
assert guide_path.is_file()
assert "docs/src/releasing/crates-io-trusted-publishing.md" in readme
assert "releasing/crates-io-trusted-publishing.md" in summary

print("PASS: trusted publishing config, release workflow, package selection, and docs links")
PY
