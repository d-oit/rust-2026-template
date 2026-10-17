#!/usr/bin/env python3
"""Parse benchmark runs into the machine-readable event format.

Two sources are merged, keyed by benchmark name:

1. `test <name> ... bench: <n> ns/iter` lines. Emitted by the Tokio harness
   (`benchmarks/benches/tokio_*_bench.rs`), which prints one row per percentile, and by
   criterion when its bencher output carries ids.
2. Criterion's own estimates at `<criterion-dir>/**/new/estimates.json`. Criterion 0.8's
   bencher output omits the benchmark id, so its rows are read from the JSON artifacts it
   always writes: name = `group/function`, value = median point estimate in nanoseconds.

Rows from source 1 win on a name collision. Without the second source the pipeline
records nothing for criterion-only targets; without the first it loses the harness
percentiles. Both are required.
"""
import os
import sys
import json
import re
import platform
import subprocess
from datetime import datetime
from pathlib import Path

# Regex to match: test benchmark_name ... bench: 1234 ns/iter (+/- 0)
BENCHER_PATTERN = re.compile(
    r"test\s+(?P<name>\S+)\s+\.\.\.\s+bench:\s+(?P<value>[\d,]+)\s+ns/iter"
)


def get_env_metadata(runner_arg=None):
    runner = (
        runner_arg
        or os.environ.get("BENCH_RUNNER_NAME")
        or os.environ.get("BENCH_REFERENCE_RUNNER")
    )
    if not runner:
        if os.environ.get("GITHUB_ACTIONS") == "true":
            runner = "github-runner"
        else:
            runner = "local-dev"

    toolchain = os.environ.get("RUSTC_VERSION")
    if not toolchain:
        try:
            toolchain = (
                subprocess.check_output(
                    ["rustc", "--version"], stderr=subprocess.DEVNULL
                )
                .decode()
                .strip()
            )
        except Exception:
            toolchain = "unknown"

    cpu = os.environ.get("BENCH_CPU_INFO")
    if not cpu:
        try:
            cpu_path = Path("/proc/cpuinfo")
            if cpu_path.exists():
                for line in cpu_path.read_text(encoding="utf-8").splitlines():
                    if "model name" in line:
                        cpu = line.split(":", 1)[1].strip()
                        break
        except Exception:
            pass
    if not cpu:
        cpu = platform.processor() or platform.machine() or "unknown"

    os_info = (
        os.environ.get("BENCH_OS_INFO")
        or f"{platform.system()} {platform.release()} ({platform.machine()})"
    )

    return {
        "runner": runner,
        "toolchain": toolchain,
        "cpu": cpu,
        "os": os_info,
    }


def parse_bench_output(output, commit_sha, timestamp, env_meta=None):
    env_meta = env_meta or {}
    results = []
    for line in output.splitlines():
        match = BENCHER_PATTERN.search(line)
        if match:
            val = int(match.group("value").replace(",", ""))
            results.append({
                "timestamp": timestamp,
                "commit": commit_sha,
                "benchmark": match.group("name"),
                "ns_per_iter": val,
                "throughput_mb_s": None,
                "runner": env_meta.get("runner", "unknown"),
                "cpu": env_meta.get("cpu", "unknown"),
                "toolchain": env_meta.get("toolchain", "unknown"),
                "os": env_meta.get("os", "unknown"),
                "median_ns": float(val),
                "confidence_interval": {
                    "lower_bound_ns": float(val),
                    "upper_bound_ns": float(val),
                    "confidence_level": 0.95,
                },
            })
    return results


def parse_criterion_estimates(criterion_dir, commit_sha, timestamp, env_meta=None):
    """Reads criterion's per-benchmark estimates from `<criterion_dir>/**/new/estimates.json`."""
    env_meta = env_meta or {}
    results = []
    root = Path(criterion_dir)
    if not root.is_dir():
        return results

    for estimate in sorted(root.glob("**/new/estimates.json")):
        name = str(estimate.parent.parent.relative_to(root))
        try:
            data = json.loads(estimate.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            continue
        statistic = data.get("median") or data.get("mean") or {}
        val = statistic.get("point_estimate")
        if val is None:
            continue

        ci = statistic.get("confidence_interval") or {}
        lower_bound = ci.get("lower_bound", val)
        upper_bound = ci.get("upper_bound", val)
        conf_level = ci.get("confidence_level", 0.95)

        results.append({
            "timestamp": timestamp,
            "commit": commit_sha,
            "benchmark": name,
            "ns_per_iter": int(round(val)),
            "throughput_mb_s": None,
            "runner": env_meta.get("runner", "unknown"),
            "cpu": env_meta.get("cpu", "unknown"),
            "toolchain": env_meta.get("toolchain", "unknown"),
            "os": env_meta.get("os", "unknown"),
            "median_ns": float(round(val, 3)),
            "confidence_interval": {
                "lower_bound_ns": float(round(lower_bound, 3)),
                "upper_bound_ns": float(round(upper_bound, 3)),
                "confidence_level": float(conf_level),
            },
        })
    return results


def merge_rows(primary, secondary):
    """Merges the two sources by benchmark name; `primary` wins collisions."""
    merged = {row["benchmark"]: row for row in secondary}
    for row in primary:
        merged[row["benchmark"]] = row
    return [merged[name] for name in sorted(merged)]


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(
            "Usage: parse_bench.py <bench-output-file> [commit-sha] [timestamp] [criterion-dir] [runner-name]"
        )
        sys.exit(1)

    input_file = sys.argv[1]
    commit_sha = sys.argv[2] if len(sys.argv) > 2 else "unknown"
    timestamp = (
        sys.argv[3]
        if len(sys.argv) > 3
        else datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ")
    )
    criterion_dir = sys.argv[4] if len(sys.argv) > 4 else "target/criterion"
    runner_arg = sys.argv[5] if len(sys.argv) > 5 else None

    env_meta = get_env_metadata(runner_arg)

    try:
        with open(input_file, "r", encoding="utf-8") as f:
            content = f.read()

        results = merge_rows(
            parse_bench_output(content, commit_sha, timestamp, env_meta),
            parse_criterion_estimates(criterion_dir, commit_sha, timestamp, env_meta),
        )
        for res in results:
            print(json.dumps(res))
    except Exception as e:
        print(f"Error parsing benchmark output: {e}", file=sys.stderr)
        sys.exit(1)
