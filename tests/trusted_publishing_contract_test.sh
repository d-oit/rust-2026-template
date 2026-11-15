#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

python3 - <<'PY'
import json
import os
import re
import subprocess
import sys
from pathlib import Path
import tomllib

import yaml

workflow_path = Path(".github/workflows/release.yml")
config_path = Path("config/trusted-publishing.json")
dist_config_path = Path("dist-workspace.toml")
workflow = yaml.safe_load(workflow_path.read_text(encoding="utf-8"))
config = json.loads(config_path.read_text(encoding="utf-8"))
dist_config = tomllib.loads(dist_config_path.read_text(encoding="utf-8"))

release_jobs = workflow["jobs"]
for job_name in ("plan", "build-global-artifacts", "host"):
    assert release_jobs[job_name]["runs-on"] == "ubuntu-24.04", (
        f"{job_name} must use the explicitly supported Ubuntu 24.04 runner"
    )
assert "ubuntu-22.04" not in workflow_path.read_text(encoding="utf-8"), (
    "release workflow must not reference the deprecated Ubuntu 22.04 image"
)
assert release_jobs["build-local-artifacts"]["runs-on"] == "${{ matrix.runner }}", (
    "cargo-dist target builds must keep their configured runner matrix"
)
custom_runners = dist_config["dist"]["github-custom-runners"]
assert custom_runners["global"] == "ubuntu-24.04", (
    "cargo-dist global artifact jobs must use Ubuntu 24.04"
)
assert custom_runners["x86_64-unknown-linux-gnu"] == "ubuntu-24.04", (
    "cargo-dist Linux x64 artifact builds must use Ubuntu 24.04"
)
assert dist_config["dist"]["targets"] == [
    "x86_64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
], "runner migration must preserve the supported release targets"

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

# Execute the exact CLI commands the workflow runs. A substring assertion is
# not enough: `--root` is a top-level flag, so `trustpub.py <cmd> --root .`
# parses fine on paper but exits 2 in production. Only running the command
# proves the workflow invocation is valid.
import shlex

parsed = yaml.safe_load(workflow_path.read_text(encoding="utf-8"))
command_steps = [
    step["run"]
    for job in parsed["jobs"].values()
    for step in job.get("steps", [])
    if "trustpub.py" in str(step.get("run", ""))
]
assert command_steps, "release workflow no longer invokes the trusted publishing CLI"

for command in command_steps:
    # Strip shell redirection so only the CLI invocation is executed.
    argv = shlex.split(command.split(">>")[0].strip())
    assert argv[0] == "python3", f"unexpected interpreter in workflow command: {argv[0]}"
    completed = subprocess.run(
        [sys.executable, *argv[1:]],
        capture_output=True,
        text=True,
        check=False,
        env={**os.environ, "CARGO_REGISTRY_TOKEN": ""},
    )
    # argparse exits 2 on a malformed invocation. That is the failure mode this
    # check exists to catch (`--root` is a top-level flag, so placing it after
    # the subcommand parses fine on paper but exits 2 in production).
    assert completed.returncode != 2, (
        f"workflow command is not parseable: {command}\n"
        f"stderr: {completed.stderr.strip()}"
    )
    assert "unrecognized arguments" not in completed.stderr, (
        f"workflow command has misplaced arguments: {command}\n"
        f"stderr: {completed.stderr.strip()}"
    )
    # Commands that read only local configuration must fully succeed offline.
    # `release-publish` legitimately exits 1 without a registry token, so it is
    # covered by the parse check above rather than a zero-exit requirement.
    if "release-publish" not in command:
        assert completed.returncode == 0, (
            f"workflow command failed with exit {completed.returncode}: {command}\n"
            f"stderr: {completed.stderr.strip()}"
        )
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
