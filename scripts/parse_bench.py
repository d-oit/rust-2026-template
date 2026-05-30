#!/usr/bin/env python3
import sys
import json
import re
from datetime import datetime

def parse_bench_output(output, commit_sha, timestamp):
    # Regex to match: test benchmark_name ... bench: 1234 ns/iter (+/- 0)
    pattern = re.compile(r"test\s+(?P<name>\S+)\s+\.\.\.\s+bench:\s+(?P<value>[\d,]+)\s+ns/iter")

    results = []
    for line in output.splitlines():
        match = pattern.search(line)
        if match:
            name = match.group("name")
            value = int(match.group("value").replace(",", ""))
            results.append({
                "timestamp": timestamp,
                "commit": commit_sha,
                "benchmark": name,
                "ns_per_iter": value,
                "throughput_mb_s": None # Bencher format doesn't easily provide throughput
            })
    return results

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: parse_bench.py <bench-output-file> [commit-sha] [timestamp]")
        sys.exit(1)

    input_file = sys.argv[1]
    commit_sha = sys.argv[2] if len(sys.argv) > 2 else "unknown"
    timestamp = sys.argv[3] if len(sys.argv) > 3 else datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ")

    try:
        with open(input_file, "r") as f:
            content = f.read()

        results = parse_bench_output(content, commit_sha, timestamp)
        for res in results:
            print(json.dumps(res))
    except Exception as e:
        print(f"Error parsing benchmark output: {e}", file=sys.stderr)
        sys.exit(1)
