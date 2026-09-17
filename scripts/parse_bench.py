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
import sys
import json
import re
from datetime import datetime
from pathlib import Path

# Regex to match: test benchmark_name ... bench: 1234 ns/iter (+/- 0)
BENCHER_PATTERN = re.compile(
    r"test\s+(?P<name>\S+)\s+\.\.\.\s+bench:\s+(?P<value>[\d,]+)\s+ns/iter"
)


def parse_bench_output(output, commit_sha, timestamp):
    results = []
    for line in output.splitlines():
        match = BENCHER_PATTERN.search(line)
        if match:
            results.append({
                "timestamp": timestamp,
                "commit": commit_sha,
                "benchmark": match.group("name"),
                "ns_per_iter": int(match.group("value").replace(",", "")),
                "throughput_mb_s": None,  # Bencher format doesn't easily provide throughput
            })
    return results


def parse_criterion_estimates(criterion_dir, commit_sha, timestamp):
    """Reads criterion's per-benchmark estimates from `<criterion_dir>/**/new/estimates.json`."""
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
        value = statistic.get("point_estimate")
        if value is None:
            continue
        results.append({
            "timestamp": timestamp,
            "commit": commit_sha,
            "benchmark": name,
            "ns_per_iter": int(round(value)),
            "throughput_mb_s": None,
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
            "Usage: parse_bench.py <bench-output-file> [commit-sha] [timestamp] [criterion-dir]"
        )
        sys.exit(1)

    input_file = sys.argv[1]
    commit_sha = sys.argv[2] if len(sys.argv) > 2 else "unknown"
    timestamp = sys.argv[3] if len(sys.argv) > 3 else datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ")
    criterion_dir = sys.argv[4] if len(sys.argv) > 4 else "target/criterion"

    try:
        with open(input_file, "r") as f:
            content = f.read()

        results = merge_rows(
            parse_bench_output(content, commit_sha, timestamp),
            parse_criterion_estimates(criterion_dir, commit_sha, timestamp),
        )
        for res in results:
            print(json.dumps(res))
    except Exception as e:
        print(f"Error parsing benchmark output: {e}", file=sys.stderr)
        sys.exit(1)
