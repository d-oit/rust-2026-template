#!/usr/bin/env python3
"""Contract validation for generated DORA report artifacts.

The generated maintenance fast path must still prove that the three committed
DORA artifacts are internally consistent. This validator is deliberately
offline and deterministic: it reads only the files in the repository and never
contacts the GitHub API, so a maintenance PR cannot pass by fetching different
data than it commits.

Cross-file consistency is the core check. The report, manifest, and history
entry for the same generation must agree on the evaluation window, the metric
values, and the tier summary; drift between them is a failing gate, not a
cosmetic nit.

Usage::

    python3 scripts/validate-dora-artifacts.py
    python3 scripts/validate-dora-artifacts.py --report reports/DORA-REPORT.md \\
        --manifest reports/dora-manifest.json --history reports/dora-history.jsonl
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

REPORT_PATH = Path("reports/DORA-REPORT.md")
MANIFEST_PATH = Path("reports/dora-manifest.json")
HISTORY_PATH = Path("reports/dora-history.jsonl")

SUPPORTED_MANIFEST_SCHEMA = "1.0"
SUPPORTED_POLICY_PERCENTILES = {"mean", "median", "p90"}
TIERS = {"Elite", "High", "Medium", "Low", "N/A", "Unknown"}
APPROVED_BADGES = {
    "Elite": "brightgreen",
    "High": "green",
    "Medium": "yellow",
    "Low": "orange",
    "Unknown": "lightgrey",
}

# Fields the generator always writes for each metric section, plus the value
# type each one carries. Anything else in a section is tolerated so the
# generator can add fields without breaking the gate.
METRIC_SCHEMA: dict[str, dict[str, type]] = {
    "deployment_frequency": {"count": int, "per_day": float, "tier": str},
    "change_lead_time": {"avg_hours": float, "tier": str},
    "change_failure_rate": {"hotfixes": int, "total": int, "rate": float, "tier": str},
    "failed_deployment_recovery_time": {"avg_hours": float, "tier": str},
}

AGENTIC_SCHEMA: dict[str, type] = {
    "total_tasks": int,
    "success_rate": float,
    "human_intervention_rate": float,
    "avg_tokens": int,
}

REQUIRED_REPORT_SECTIONS = (
    "# DORA Delivery Performance Report",
    "## Core DORA Metrics",
    "## Agentic Harness Metrics",
    "## DORA Tier Summary",
    "## Derivation Manifest",
    "## Trend: Latest 3 Weekly Reports",
)

ISO_UTC = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
BADGE = re.compile(
    r"!\[(?P<tier>[A-Za-z]+)\]"
    r"\(https://img\.shields\.io/badge/DORA-(?P=tier)-(?P<badge>[a-z]+)\)"
)

REQUIRED_INPUT_HASHES = ("releases", "prs", "agent_metrics", "dora_metrics", "policy")
REQUIRED_POLICY_FIELDS = (
    "policy_file",
    "bot_allowlist",
    "merge_strategy",
    "percentile_method",
    "revert_predicate",
)


class ArtifactError(ValueError):
    """A generated DORA artifact violates its contract."""


def parse_timestamp(value: str, label: str) -> datetime:
    """Parse the generator's canonical `YYYY-MM-DDTHH:MM:SSZ` UTC format."""
    if not isinstance(value, str) or not ISO_UTC.match(value):
        raise ArtifactError(f"{label} must be UTC 'YYYY-MM-DDTHH:MM:SSZ', got {value!r}")
    try:
        return datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)
    except ValueError as error:
        raise ArtifactError(f"{label} is not a real timestamp: {value!r} ({error})") from error


def require(condition: bool, message: str) -> None:
    """Raise `ArtifactError` when `condition` is false."""
    if not condition:
        raise ArtifactError(message)


def check_type(section: dict[str, Any], field: str, expected: type, where: str) -> Any:
    """Validate one field's presence and JSON type, rejecting bool-as-int coercion.

    JSON has a single number type, and the generator round-trips integral values
    such as `rate: 0` as integers. A `float` field therefore accepts either JSON
    number, but never `bool` (which subclasses `int` in Python) or a string.
    """
    require(field in section, f"{where} is missing required field {field!r}")
    value = section[field]
    allowed: tuple[type, ...] = (float, int) if expected is float else (expected,)
    if isinstance(value, bool) and bool not in allowed:
        raise ArtifactError(f"{where}.{field} must be {expected.__name__}, got bool")
    require(
        isinstance(value, allowed),
        f"{where}.{field} must be {expected.__name__}, got {type(value).__name__}",
    )
    return value


def validate_manifest(manifest: Any) -> dict[str, Any]:
    """Validate the derivation manifest's schema, types, and policy provenance."""
    require(isinstance(manifest, dict), "manifest must be a JSON object")

    version = manifest.get("schema_version")
    require(
        version == SUPPORTED_MANIFEST_SCHEMA,
        f"manifest schema_version {version!r} is unsupported (expected {SUPPORTED_MANIFEST_SCHEMA!r})",
    )

    window = manifest.get("evaluation_window")
    require(isinstance(window, dict), "manifest.evaluation_window must be an object")
    now_iso = check_type(window, "now_iso", str, "manifest.evaluation_window")
    start_iso = check_type(window, "start_iso", str, "manifest.evaluation_window")
    period_days = check_type(window, "period_days", int, "manifest.evaluation_window")
    now_dt = parse_timestamp(now_iso, "manifest.evaluation_window.now_iso")
    start_dt = parse_timestamp(start_iso, "manifest.evaluation_window.start_iso")
    require(period_days > 0, "manifest.evaluation_window.period_days must be positive")
    require(start_dt < now_dt, "manifest evaluation window must have start_iso before now_iso")
    drift = abs((now_dt - start_dt).days - period_days)
    require(
        drift <= 1,
        f"evaluation window spans {(now_dt - start_dt).days} days but declares period_days={period_days}",
    )

    hashes = manifest.get("input_hashes")
    require(isinstance(hashes, dict), "manifest.input_hashes must be an object")
    for name in REQUIRED_INPUT_HASHES:
        value = check_type(hashes, name, str, "manifest.input_hashes")
        require(
            SHA256.match(value) is not None,
            f"manifest.input_hashes.{name} must be a lowercase SHA-256 hex digest",
        )

    policy = manifest.get("policy")
    require(isinstance(policy, dict), "manifest.policy must be an object")
    for field in REQUIRED_POLICY_FIELDS:
        require(field in policy, f"manifest.policy is missing required field {field!r}")
    require(
        isinstance(policy["policy_file"], (str, type(None))),
        "manifest.policy.policy_file must be a string or null",
    )
    require(
        isinstance(policy["bot_allowlist"], list)
        and all(isinstance(entry, str) for entry in policy["bot_allowlist"]),
        "manifest.policy.bot_allowlist must be a list of strings",
    )
    for field in ("merge_strategy", "percentile_method", "revert_predicate"):
        require(
            isinstance(policy[field], str) and policy[field],
            f"manifest.policy.{field} must be a non-empty string",
        )
    require(
        policy["percentile_method"] in SUPPORTED_POLICY_PERCENTILES,
        f"manifest.policy.percentile_method {policy['percentile_method']!r} is unsupported",
    )

    counts = manifest.get("scanned_counts")
    require(isinstance(counts, dict), "manifest.scanned_counts must be an object")
    for name in ("releases", "prs", "agent_metrics", "dora_metrics"):
        value = check_type(counts, name, int, "manifest.scanned_counts")
        require(value >= 0, f"manifest.scanned_counts.{name} must not be negative")

    metrics = manifest.get("metrics")
    require(isinstance(metrics, dict), "manifest.metrics must be an object")
    for name, fields in METRIC_SCHEMA.items():
        section = metrics.get(name)
        require(isinstance(section, dict), f"manifest.metrics.{name} must be an object")
        for field, expected in fields.items():
            check_type(section, field, expected, f"manifest.metrics.{name}")
        tier = section["tier"]
        require(tier in TIERS, f"manifest.metrics.{name}.tier {tier!r} is not a DORA tier")

    agentic = metrics.get("agentic_metrics")
    require(isinstance(agentic, dict), "manifest.metrics.agentic_metrics must be an object")
    for field, expected in AGENTIC_SCHEMA.items():
        check_type(agentic, field, expected, "manifest.metrics.agentic_metrics")

    overall = metrics.get("overall_tier")
    require(overall in TIERS, f"manifest.metrics.overall_tier {overall!r} is not a DORA tier")
    require(
        overall == expected_overall_tier(metrics),
        f"manifest.metrics.overall_tier {overall!r} disagrees with the per-metric tiers",
    )

    return manifest


def expected_overall_tier(metrics: dict[str, Any]) -> str:
    """Recompute the generator's worst-tier aggregation for cross-checking."""
    priority = {"Elite": 4, "High": 3, "Medium": 2, "Low": 1, "N/A": 0}
    considered = [
        metrics[name]["tier"]
        for name in ("deployment_frequency", "change_lead_time", "change_failure_rate",
                     "failed_deployment_recovery_time")
    ]
    real = [tier for tier in considered if tier != "N/A"]
    if not real:
        return "Unknown"
    worst = min(priority[tier] for tier in real)
    return next(name for name, value in priority.items() if value == worst)


def validate_history(path: Path) -> list[dict[str, Any]]:
    """Validate every non-empty JSONL line and require strictly ordered records."""
    if not path.is_file():
        raise ArtifactError(f"history file is missing: {path}")
    lines = [line for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]
    require(bool(lines), f"history file is empty: {path} (the generator must emit a snapshot)")

    records: list[dict[str, Any]] = []
    previous: datetime | None = None
    for lineno, line in enumerate(lines, start=1):
        try:
            record = json.loads(line)
        except json.JSONDecodeError as error:
            raise ArtifactError(f"{path}:{lineno} is not valid JSON: {error}") from error
        require(isinstance(record, dict), f"{path}:{lineno} must be a JSON object")

        generated_at = parse_timestamp(
            check_type(record, "generated_at", str, f"{path}:{lineno}"),
            f"{path}:{lineno}.generated_at",
        )
        period_days = check_type(record, "period_days", int, f"{path}:{lineno}")
        require(period_days > 0, f"{path}:{lineno}.period_days must be positive")

        metrics = record.get("metrics")
        require(isinstance(metrics, dict), f"{path}:{lineno}.metrics must be an object")
        for name, fields in METRIC_SCHEMA.items():
            section = metrics.get(name)
            require(
                isinstance(section, dict),
                f"{path}:{lineno}.metrics.{name} must be an object (missing metric section)",
            )
            for field, expected in fields.items():
                check_type(section, field, expected, f"{path}:{lineno}.metrics.{name}")
            require(
                section["tier"] in TIERS,
                f"{path}:{lineno}.metrics.{name}.tier {section['tier']!r} is not a DORA tier",
            )

        if previous is not None:
            require(
                generated_at >= previous,
                f"{path}:{lineno} is older than the previous record; history must be chronological",
            )
        previous = generated_at
        records.append(record)
    return records


def validate_report(path: Path) -> str:
    """Validate the Markdown report's required structure, period, and metric table."""
    if not path.is_file():
        raise ArtifactError(f"report is missing: {path}")
    text = path.read_text(encoding="utf-8")
    for section in REQUIRED_REPORT_SECTIONS:
        require(section in text, f"report is missing required section {section!r}")

    generated = re.search(r"^\*\*Generated:\*\* (?P<value>\S+)$", text, re.MULTILINE)
    require(generated is not None, "report is missing a '**Generated:** <timestamp>' line")
    parse_timestamp(generated.group("value"), "report Generated timestamp")

    period = re.search(r"^\*\*Period:\*\* Last (?P<days>\d+) days$", text, re.MULTILINE)
    require(period is not None, "report is missing a '**Period:** Last N days' line")
    require(int(period.group("days")) > 0, "report period must be a positive number of days")

    repository = re.search(r"^\*\*Repository:\*\* (?P<repo>\S+/\S+)$", text, re.MULTILINE)
    require(repository is not None, "report is missing a '**Repository:** owner/name' line")

    metric_rows = {
        label: {"value": value, "tier": tier}
        for label, value, tier in re.findall(
            r"^\| (?P<label>[A-Za-z ]+) \| (?P<value>.*?) \| \*\*(?P<tier>[A-Za-z/]+)\*\* \|$",
            text,
            re.MULTILINE,
        )
    }
    for label in (
        "Deployment Frequency",
        "Change Lead Time",
        "Change Failure Rate",
        "Failed Deployment Recovery Time",
    ):
        require(label in metric_rows, f"report is missing the {label!r} metric row")

    badge = BADGE.search(text)
    require(badge is not None, "report is missing a DORA tier badge")
    tier, badge_colour = badge.group("tier"), badge.group("badge")
    require(tier in APPROVED_BADGES, f"report badge tier {tier!r} is not a DORA tier")
    require(
        APPROVED_BADGES[tier] == badge_colour,
        f"report badge for {tier!r} uses colour {badge_colour!r}",
    )

    trend_header = "| Report date (UTC) | Window (days) | Deployment frequency |"
    require(trend_header in text, "report trend table header is missing or reformatted")

    return text


def validate_cross_file(
    manifest: dict[str, Any],
    history: list[dict[str, Any]],
    report_text: str,
) -> None:
    """Require the report, manifest, and latest history entry to describe one generation."""
    now_iso = manifest["evaluation_window"]["now_iso"]
    period_days = manifest["evaluation_window"]["period_days"]
    metrics = manifest["metrics"]

    latest = history[-1]
    require(
        latest["generated_at"] == now_iso,
        f"latest history snapshot ({latest['generated_at']}) does not match manifest now_iso ({now_iso})",
    )
    require(
        latest["period_days"] == period_days,
        f"latest history period_days ({latest['period_days']}) does not match manifest ({period_days})",
    )
    for name in METRIC_SCHEMA:
        for field in METRIC_SCHEMA[name]:
            require(
                latest["metrics"][name][field] == metrics[name][field],
                f"latest history {name}.{field} ({latest['metrics'][name][field]!r}) "
                f"does not match manifest ({metrics[name][field]!r})",
            )

    require(
        f"**Generated:** {now_iso}" in report_text,
        f"report Generated timestamp does not match manifest now_iso ({now_iso})",
    )
    require(
        f"**Period:** Last {period_days} days" in report_text,
        f"report period does not match manifest period_days ({period_days})",
    )

    df = metrics["deployment_frequency"]
    require(
        f"| Deployment Frequency | {df['per_day']}/day ({df['count']} releases) | **{df['tier']}** |"
        in report_text,
        "report Deployment Frequency row disagrees with the manifest",
    )
    clt = metrics["change_lead_time"]
    require(
        f"| Change Lead Time | {clt['avg_hours']} hours avg | **{clt['tier']}** |" in report_text,
        "report Change Lead Time row disagrees with the manifest",
    )
    cfr = metrics["change_failure_rate"]
    rate_percent = round(cfr["rate"] * 100, 1)
    require(
        f"| Change Failure Rate | {rate_percent}% ({cfr['hotfixes']}/{cfr['total']}) | **{cfr['tier']}** |"
        in report_text,
        "report Change Failure Rate row disagrees with the manifest",
    )
    fdrt = metrics["failed_deployment_recovery_time"]
    require(
        f"| Failed Deployment Recovery Time | {fdrt['avg_hours']} hours avg | **{fdrt['tier']}** |"
        in report_text,
        "report Failed Deployment Recovery Time row disagrees with the manifest",
    )

    agentic = metrics["agentic_metrics"]
    require(
        f"| Total agent tasks | {agentic['total_tasks']} |" in report_text,
        "report agent task count disagrees with the manifest",
    )
    require(
        f"| Agent success rate | {round(agentic['success_rate'] * 100, 1)}% |" in report_text,
        "report agent success rate disagrees with the manifest",
    )

    require(
        f"![{metrics['overall_tier']}]" in report_text,
        "report badge tier disagrees with the manifest overall_tier",
    )
    require(
        now_iso[:10] in report_text,
        "report trend table does not include the current report date",
    )


def main(argv: list[str] | None = None) -> int:
    """Validate all three artifacts, exiting non-zero on any contract violation."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--report", type=Path, default=REPORT_PATH)
    parser.add_argument("--manifest", type=Path, default=MANIFEST_PATH)
    parser.add_argument("--history", type=Path, default=HISTORY_PATH)
    args = parser.parse_args(argv)

    try:
        if not args.manifest.is_file():
            raise ArtifactError(f"manifest is missing: {args.manifest}")
        try:
            manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
        except json.JSONDecodeError as error:
            raise ArtifactError(f"manifest is not valid JSON: {error}") from error

        validate_manifest(manifest)
        history = validate_history(args.history)
        report_text = validate_report(args.report)
        validate_cross_file(manifest, history, report_text)
    except ArtifactError as error:
        print(f"::error::DORA artifact contract violation: {error}", file=sys.stderr)
        return 1

    print(
        "PASS: DORA report, manifest, and history are structurally valid "
        f"and mutually consistent (generated {manifest['evaluation_window']['now_iso']})."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
