#!/usr/bin/env python3
"""Manage crates.io Trusted Publishing for the explicitly configured workspace crates.

Read-only commands (`plan`, `list`, `verify`, `release-plan`) never mutate state.
Mutating commands (`configure`, `remove`, `bootstrap`, `require`, `release-publish`)
require explicit confirmation and fail closed when their preconditions are unmet.

The API token is read only from CARGO_REGISTRY_TOKEN and is never printed,
persisted, or passed as a process argument. Use a token with the
`trusted-publishing` endpoint scope (plus `publish-new` for the one-time
bootstrap publish).
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shlex
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

API_BASE = os.environ.get("TRUSTPUB_API_URL", "https://crates.io").rstrip("/")
TOKEN_ENV = "CARGO_REGISTRY_TOKEN"
CONFIG_NAME = "config/trusted-publishing.json"
CONFIG_KEYS = {
    "repository_owner",
    "repository_name",
    "workflow_filename",
    "environment",
    "packages",
}


class TrustpubError(Exception):
    """A user-facing failure. Messages never include credentials."""


def repo_root(explicit: str | None) -> Path:
    if explicit:
        return Path(explicit).resolve()
    if os.environ.get("TRUSTPUB_ROOT"):
        return Path(os.environ["TRUSTPUB_ROOT"]).resolve()
    try:
        output = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
    except (OSError, subprocess.CalledProcessError) as error:
        raise TrustpubError("not inside a Git repository; pass --root") from error
    return Path(output).resolve()


def load_config(root: Path, explicit: str | None) -> dict:
    explicit = explicit or os.environ.get("TRUSTPUB_CONFIG")
    path = Path(explicit) if explicit else root / CONFIG_NAME
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise TrustpubError(f"cannot read trusted-publishing config: {path}") from error
    except json.JSONDecodeError as error:
        raise TrustpubError(f"invalid JSON in {path}: {error}") from error
    if not isinstance(data, dict):
        raise TrustpubError(f"{path} must contain a JSON object")
    unknown = set(data) - CONFIG_KEYS
    if unknown:
        raise TrustpubError(f"{path} has unknown keys: {sorted(unknown)}")
    for key in ("workflow_filename", "environment"):
        value = data.get(key)
        if not isinstance(value, str) or not value.strip():
            raise TrustpubError(f"{path}: {key} must be a non-empty string")
    if data["workflow_filename"] != Path(data["workflow_filename"]).name:
        raise TrustpubError(f"{path}: workflow_filename must be a file name, not a path")
    packages = data.get("packages")
    if not isinstance(packages, list) or not all(
        isinstance(item, str) and item for item in packages
    ):
        raise TrustpubError(f"{path}: packages must be a list of crate names")
    if len(packages) != len(set(packages)):
        raise TrustpubError(f"{path}: packages contains duplicates")
    return data


def load_metadata(root: Path) -> dict[str, dict]:
    manifest = root / "Cargo.toml"
    if not manifest.is_file():
        raise TrustpubError(f"workspace manifest not found: {manifest}")
    try:
        output = subprocess.run(
            [
                "cargo",
                "metadata",
                "--no-deps",
                "--format-version",
                "1",
                "--manifest-path",
                str(manifest),
            ],
            check=True,
            capture_output=True,
            text=True,
        ).stdout
    except (OSError, subprocess.CalledProcessError) as error:
        raise TrustpubError("cargo metadata failed; run it manually for details") from error
    packages = json.loads(output)["packages"]
    return {package["name"]: package for package in packages}


def publishable(package: dict) -> bool:
    publish = package.get("publish")
    return publish is None or "crates-io" in publish


def ordered_packages(config: dict, metadata: dict[str, dict]) -> list[str]:
    selected = config["packages"]
    for name in selected:
        if name not in metadata:
            raise TrustpubError(f"allowlisted crate is not a workspace member: {name}")
        if not publishable(metadata[name]):
            raise TrustpubError(f"allowlisted crate has publish = false: {name}")
        if metadata[name]["version"] == "0.0.0":
            raise TrustpubError(
                f"allowlisted crate still uses the template placeholder version 0.0.0: {name}"
            )

    selected_set = set(selected)
    remaining = {name: set() for name in selected}
    for name in selected:
        for dependency in metadata[name].get("dependencies", []):
            if dependency.get("source") is None and dependency["name"] in selected_set:
                remaining[name].add(dependency["name"])

    order: list[str] = []
    pending = dict(remaining)
    while pending:
        ready = sorted(name for name, deps in pending.items() if not deps)
        if not ready:
            raise TrustpubError(
                "allowlisted crates contain a dependency cycle: "
                + ", ".join(sorted(pending))
            )
        for name in ready:
            order.append(name)
            pending.pop(name)
        for deps in pending.values():
            deps.difference_update(ready)
    return order


def git_identity(root: Path) -> tuple[str | None, str | None]:
    try:
        remote = subprocess.run(
            ["git", "-C", str(root), "config", "--get", "remote.origin.url"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return None, None
    match = re.search(r"github\.com[:/]([^/]+)/([^/]+?)(?:\.git)?$", remote)
    if not match:
        return None, None
    return match.group(1), match.group(2)


def identity(config: dict, args: argparse.Namespace, root: Path) -> tuple[str, str]:
    owner = args.owner or config.get("repository_owner") or os.environ.get(
        "TRUSTPUB_REPOSITORY_OWNER"
    )
    name = args.repo or config.get("repository_name") or os.environ.get(
        "TRUSTPUB_REPOSITORY_NAME"
    )
    if not owner or not name:
        git_owner, git_name = git_identity(root)
        owner = owner or git_owner
        name = name or git_name
    if not owner or not name:
        raise TrustpubError(
            "repository identity is unknown; set it in the config or pass --owner/--repo"
        )
    return owner, name


def token() -> str:
    value = os.environ.get(TOKEN_ENV)
    if not value:
        raise TrustpubError(
            f"{TOKEN_ENV} is not set; create a crates.io token with the "
            "`trusted-publishing` endpoint scope"
        )
    return value


def api(
    method: str,
    path: str,
    auth: str | None,
    body: dict | None = None,
    allow_status: tuple[int, ...] = (),
) -> tuple[int, object | None]:
    data = None
    if body is not None:
        data = json.dumps(body).encode("utf-8")
    request = urllib.request.Request(API_BASE + path, data=data, method=method)
    if auth is not None:
        request.add_header("Authorization", f"Bearer {auth}")
    request.add_header("User-Agent", "rust-2026-template-trustpub")
    if data is not None:
        request.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(request) as response:
            raw = response.read()
            return response.status, json.loads(raw) if raw else None
    except urllib.error.HTTPError as error:
        raw = error.read()
        detail = ""
        try:
            parsed = json.loads(raw)
            errors = parsed.get("errors") if isinstance(parsed, dict) else None
            if isinstance(errors, list) and errors:
                detail = str(errors[0].get("detail", ""))
        except (json.JSONDecodeError, AttributeError, TypeError):
            detail = ""
        if error.code in allow_status:
            return error.code, None
        hint = {
            401: "the token is invalid or expired",
            403: "the token lacks the required scope or crate ownership",
            404: "the crate does not exist yet or the config id is wrong",
            409: "the crate already has the maximum number of configurations",
            429: "rate limited by crates.io; retry after the indicated delay",
        }.get(error.code, "")
        message = f"crates.io API responded {error.code}"
        if hint:
            message += f" ({hint})"
        if detail:
            message += f": {detail}"
        raise TrustpubError(message) from None
    except urllib.error.URLError as error:
        raise TrustpubError(f"cannot reach {API_BASE}: {error.reason}") from None


def list_configs(crate: str, auth: str) -> list[dict]:
    configs: list[dict] = []
    path = f"/api/v1/trusted_publishing/github_configs?crate={urllib.parse.quote(crate)}"
    for _ in range(100):
        status, payload = api("GET", path, auth)
        if status != 200 or not isinstance(payload, dict):
            raise TrustpubError("unexpected response while listing trusted publishers")
        configs.extend(payload.get("github_configs") or [])
        next_page = (payload.get("meta") or {}).get("next_page")
        if not next_page:
            return configs
        path = "/api/v1/trusted_publishing/github_configs" + str(next_page)
    raise TrustpubError("trusted publisher listing exceeded 100 pages")


def matches(config: dict, expected: dict) -> bool:
    return (
        config["crate"] == expected["crate"]
        and config["repository_owner"].lower() == expected["repository_owner"].lower()
        and config["repository_name"].lower() == expected["repository_name"].lower()
        and config["workflow_filename"] == expected["workflow_filename"]
        and config.get("environment") == expected["environment"]
    )


def tuple_for(crate: str, owner: str, name: str, config: dict) -> dict:
    return {
        "crate": crate,
        "repository_owner": owner,
        "repository_name": name,
        "workflow_filename": config["workflow_filename"],
        "environment": config["environment"],
    }


def confirm(args: argparse.Namespace, action: str) -> None:
    if args.yes:
        return
    if not sys.stdin.isatty():
        raise TrustpubError(f"{action} requires --yes in non-interactive use")
    answer = input(f"{action}? Type 'yes' to continue: ").strip().lower()
    if answer != "yes":
        raise TrustpubError("aborted")


def select_packages(config: dict, args: argparse.Namespace) -> list[str]:
    if not args.package and not args.all_configured:
        raise TrustpubError("pass --package <name> (repeatable) or --all-configured")
    if args.package and args.all_configured:
        raise TrustpubError("--package and --all-configured are mutually exclusive")
    return list(args.package) if args.package else list(config["packages"])


def selected_order(config: dict, metadata: dict[str, dict], names: list[str]) -> list[str]:
    scoped = dict(config)
    scoped["packages"] = names
    return ordered_packages(scoped, metadata)


def cmd_plan(args: argparse.Namespace) -> None:
    root = repo_root(args.root)
    config = load_config(root, args.config)
    metadata = load_metadata(root)
    owner, name = identity(config, args, root)
    order = ordered_packages(config, metadata)
    print("Trusted publishing plan (read-only):")
    for crate in order:
        package = metadata[crate]
        print(
            f"  {crate} {package['version']} -> "
            f"{owner}/{name} workflow={config['workflow_filename']} "
            f"environment={config['environment']}"
        )
    if not order:
        print("  no crates in the publish allowlist; nothing will be published")


def cmd_list(args: argparse.Namespace) -> None:
    root = repo_root(args.root)
    config = load_config(root, args.config)
    metadata = load_metadata(root)
    names = selected_order(config, metadata, select_packages(config, args))
    if not names:
        print("publish allowlist is empty; nothing to list")
        return
    auth = token()
    for crate in names:
        configs = list_configs(crate, auth)
        if not configs:
            print(f"{crate}: no trusted publishers configured")
        for item in configs:
            print(
                f"{crate}: id={item['id']} "
                f"{item['repository_owner']}/{item['repository_name']} "
                f"workflow={item['workflow_filename']} "
                f"environment={item.get('environment')}"
            )


def cmd_configure(args: argparse.Namespace) -> None:
    root = repo_root(args.root)
    config = load_config(root, args.config)
    metadata = load_metadata(root)
    owner, name = identity(config, args, root)
    names = selected_order(config, metadata, select_packages(config, args))
    if not names:
        print("publish allowlist is empty; nothing to configure")
        return
    confirm(args, f"configure trusted publishers for {', '.join(names)}")
    auth = token()
    for crate in names:
        expected = tuple_for(crate, owner, name, config)
        current = list_configs(crate, auth)
        exact = [item for item in current if matches(item, expected)]
        if exact:
            print(f"{crate}: already configured (id={exact[0]['id']})")
            continue
        if current and not args.replace:
            print(f"{crate}: existing configuration differs:", file=sys.stderr)
            for item in current:
                print(f"  id={item['id']} {item}", file=sys.stderr)
            raise TrustpubError(
                f"{crate}: refusing to replace existing configuration without --replace"
            )
        if current and args.replace:
            for item in current:
                status, _ = api(
                    "DELETE",
                    f"/api/v1/trusted_publishing/github_configs/{item['id']}",
                    auth,
                )
                if status != 204:
                    raise TrustpubError(f"{crate}: failed to delete config id={item['id']}")
                print(f"{crate}: removed mismatched config id={item['id']}")
        status, payload = api(
            "POST",
            "/api/v1/trusted_publishing/github_configs",
            auth,
            {"github_config": expected},
        )
        if status != 200 or not isinstance(payload, dict):
            raise TrustpubError(f"{crate}: trusted publisher creation failed")
        created_id = payload.get("github_config", {}).get("id")
        verified = [
            item for item in list_configs(crate, auth) if matches(item, expected)
        ]
        if not verified:
            raise TrustpubError(f"{crate}: registration could not be verified after creation")
        print(f"{crate}: configured (id={created_id})")


def cmd_verify(args: argparse.Namespace) -> None:
    root = repo_root(args.root)
    config = load_config(root, args.config)
    metadata = load_metadata(root)
    owner, name = identity(config, args, root)
    names = selected_order(config, metadata, select_packages(config, args))
    if not names:
        print("publish allowlist is empty; nothing to verify")
        return
    auth = token()
    failures: list[str] = []
    for crate in names:
        expected = tuple_for(crate, owner, name, config)
        current = list_configs(crate, auth)
        if any(matches(item, expected) for item in current):
            print(f"{crate}: verified")
        else:
            failures.append(crate)
            print(f"{crate}: MISSING expected trusted publisher", file=sys.stderr)
    if failures:
        raise TrustpubError(
            "verification failed for: " + ", ".join(failures)
        )


def cmd_remove(args: argparse.Namespace) -> None:
    confirm(args, f"remove trusted publisher config id={args.id}")
    auth = token()
    status, _ = api(
        "DELETE", f"/api/v1/trusted_publishing/github_configs/{args.id}", auth
    )
    if status != 204:
        raise TrustpubError(f"failed to remove config id={args.id}")
    print(f"removed trusted publisher config id={args.id}")


def run_cargo(args: list[str], cwd: Path) -> None:
    try:
        result = subprocess.run(args, cwd=cwd, check=False)
    except OSError as error:
        raise TrustpubError(f"failed to execute {args[0]}") from error
    if result.returncode != 0:
        raise TrustpubError(f"command failed with exit code {result.returncode}: {args[0]}")


def wait_for_version(crate: str, version: str) -> None:
    attempts = int(os.environ.get("TRUSTPUB_POLL_ATTEMPTS", "10"))
    delay = float(os.environ.get("TRUSTPUB_INDEX_WAIT_SECONDS", "30"))
    path = f"/api/v1/crates/{urllib.parse.quote(crate)}/{version}"
    for attempt in range(attempts):
        status, _ = api("GET", path, None, allow_status=(404,))
        if status == 200:
            return
        if attempt + 1 == attempts:
            raise TrustpubError(
                f"{crate} {version} was published but is not visible yet; "
                f"do not republish. Run: ./scripts/trustpub.sh configure "
                f"--package {crate} --yes"
            )
        if delay:
            time.sleep(delay)


def cmd_bootstrap(args: argparse.Namespace) -> None:
    root = repo_root(args.root)
    config = load_config(root, args.config)
    metadata = load_metadata(root)
    owner, name = identity(config, args, root)
    crate = args.package
    if crate not in config["packages"]:
        raise TrustpubError(f"{crate}: not in the publish allowlist")
    package = metadata.get(crate)
    if package is None:
        raise TrustpubError(f"{crate}: not a workspace member")
    if not publishable(package):
        raise TrustpubError(f"{crate}: package has publish = false")
    if package["version"] == "0.0.0" and not args.allow_placeholder_version:
        raise TrustpubError(
            f"{crate}: version 0.0.0 is a template placeholder; "
            "pass --allow-placeholder-version only after a reviewed override"
        )
    print(
        f"About to publish {crate} {package['version']} from {root} "
        f"(initial crates.io publication), then register "
        f"{owner}/{name} workflow={config['workflow_filename']} "
        f"environment={config['environment']}"
    )
    confirm(args, "publish the initial crate version")
    auth = token()

    package_args = os.environ.get("TRUSTPUB_CARGO_PACKAGE")
    package_cmd = shlex.split(package_args) if package_args else ["cargo", "package"]
    run_cargo(package_cmd + ["--list", "--allow-dirty", "-p", crate], root)

    publish_args = os.environ.get("TRUSTPUB_CARGO_PUBLISH")
    publish_cmd = shlex.split(publish_args) if publish_args else ["cargo", "publish"]
    run_cargo(publish_cmd + ["--locked", "--package", crate], root)

    wait_for_version(crate, package["version"])

    expected = tuple_for(crate, owner, name, config)
    try:
        status, payload = api(
            "POST",
            "/api/v1/trusted_publishing/github_configs",
            auth,
            {"github_config": expected},
        )
        if status != 200 or not isinstance(payload, dict):
            raise TrustpubError("registration request was rejected")
        verified = [item for item in list_configs(crate, auth) if matches(item, expected)]
        if not verified:
            raise TrustpubError("registration could not be verified")
    except TrustpubError as error:
        raise TrustpubError(
            f"{crate} {package['version']} is published, but publisher registration failed: "
            f"{error}. Do not publish again. Retry with: "
            f"./scripts/trustpub.sh configure --package {crate} --yes"
        ) from None
    print(f"{crate}: published and trusted publisher registered")


def cmd_require(args: argparse.Namespace) -> None:
    root = repo_root(args.root)
    config = load_config(root, args.config)
    metadata = load_metadata(root)
    owner, name = identity(config, args, root)
    crate = args.package
    ordered_packages({**config, "packages": [crate]}, metadata)
    expected = tuple_for(crate, owner, name, config)
    auth = token()
    if args.enabled:
        if not args.confirm_oidc_publish:
            raise TrustpubError(
                "refusing to enable trusted-publishing-only before confirming a "
                "successful OIDC publish; pass --confirm-oidc-publish"
            )
        current = list_configs(crate, auth)
        if not any(matches(item, expected) for item in current):
            raise TrustpubError(
                f"{crate}: expected trusted publisher is missing; run configure first"
            )
    confirm(
        args,
        f"{'enable' if args.enabled else 'disable'} trusted-publishing-only for {crate}",
    )
    body = {"crate": {"trustpub_only": args.enabled}}
    status, _ = api("PATCH", f"/api/v1/crates/{urllib.parse.quote(crate)}", auth, body)
    if status != 200:
        raise TrustpubError(f"{crate}: failed to update trustpub_only")
    status, payload = api("GET", f"/api/v1/crates/{urllib.parse.quote(crate)}", auth)
    if status != 200 or not isinstance(payload, dict):
        raise TrustpubError(f"{crate}: failed to re-fetch crate metadata")
    effective = (payload.get("crate") or {}).get("trustpub_only")
    if effective is not args.enabled:
        raise TrustpubError(
            f"{crate}: trustpub_only is {effective!r}, expected {args.enabled!r}"
        )
    print(f"{crate}: trustpub_only={str(args.enabled).lower()}")


def cmd_release_plan(args: argparse.Namespace) -> None:
    root = repo_root(args.root)
    config = load_config(root, args.config)
    metadata = load_metadata(root)
    order = ordered_packages(config, metadata)
    print(f"has_packages={'true' if order else 'false'}")
    print(f"packages={' '.join(order)}")


def cmd_release_publish(args: argparse.Namespace) -> None:
    root = repo_root(args.root)
    config = load_config(root, args.config)
    metadata = load_metadata(root)
    order = ordered_packages(config, metadata)
    if not order:
        print("publish allowlist is empty; nothing to publish")
        return
    token()  # cargo publish reads CARGO_REGISTRY_TOKEN from the environment
    for crate in order:
        version = metadata[crate]["version"]
        status, _ = api(
            "GET",
            f"/api/v1/crates/{urllib.parse.quote(crate)}/{version}",
            None,
            allow_status=(404,),
        )
        if status == 200:
            print(f"{crate} {version}: already present on crates.io; skipping")
            continue
        print(f"{crate} {version}: publishing")
        publish_args = os.environ.get("TRUSTPUB_CARGO_PUBLISH")
        publish_cmd = shlex.split(publish_args) if publish_args else ["cargo", "publish"]
        run_cargo(publish_cmd + ["--locked", "--package", crate], root)
        wait_for_version(crate, version)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description=(
            "Manage crates.io Trusted Publishing. Tokens come only from "
            f"{TOKEN_ENV} (endpoint scope: trusted-publishing; add publish-new for "
            "the one-time initial publish). Cargo has no stable `cargo trustpub` "
            "subcommand; use this tool."
        )
    )
    parser.add_argument("--root", help="repository root (default: Git top level)")
    parser.add_argument("--config", help=f"config path (default: {CONFIG_NAME})")
    parser.add_argument("--owner", help="GitHub repository owner override")
    parser.add_argument("--repo", help="GitHub repository name override")
    sub = parser.add_subparsers(dest="command", required=True)

    def add(name: str, func, help_text: str) -> argparse.ArgumentParser:
        item = sub.add_parser(name, help=help_text)
        item.add_argument("--package", action="append", help="crate name (repeatable)")
        item.add_argument(
            "--all-configured",
            action="store_true",
            help="select every crate in the publish allowlist",
        )
        item.add_argument("--yes", action="store_true", help="confirm mutations")
        item.set_defaults(func=func)
        return item

    add("plan", cmd_plan, "print the read-only configuration plan")
    add("list", cmd_list, "list current publisher configurations")
    configure = add("configure", cmd_configure, "register missing publisher configurations")
    configure.add_argument(
        "--replace", action="store_true", help="replace mismatched configurations"
    )
    add("verify", cmd_verify, "verify every configured crate matches the expected tuple")

    remove = sub.add_parser("remove", help="delete a configuration by id")
    remove.add_argument("--id", required=True, type=int)
    remove.add_argument("--yes", action="store_true")
    remove.set_defaults(func=cmd_remove)

    bootstrap = sub.add_parser("bootstrap", help="one-time initial publication")
    bootstrap.add_argument("--package", required=True)
    bootstrap.add_argument("--yes", action="store_true")
    bootstrap.add_argument(
        "--allow-placeholder-version",
        action="store_true",
        help="allow publishing version 0.0.0 (reviewed override only)",
    )
    bootstrap.set_defaults(func=cmd_bootstrap)

    require = sub.add_parser("require", help="toggle trusted-publishing-only enforcement")
    require.add_argument("--package", required=True)
    group = require.add_mutually_exclusive_group(required=True)
    group.add_argument("--enabled", action="store_true")
    group.add_argument("--disabled", action="store_true")
    require.add_argument("--yes", action="store_true")
    require.add_argument(
        "--confirm-oidc-publish",
        action="store_true",
        help="confirm a successful OIDC publish before enabling enforcement",
    )
    require.set_defaults(func=cmd_require, enabled=None)

    release_plan = sub.add_parser("release-plan", help="emit GitHub Actions outputs")
    release_plan.set_defaults(func=cmd_release_plan)
    release_publish = sub.add_parser(
        "release-publish", help="publish allowlisted crates in dependency order"
    )
    release_publish.set_defaults(func=cmd_release_publish)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    if args.command == "require":
        args.enabled = bool(args.enabled)
    try:
        args.func(args)
    except TrustpubError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
