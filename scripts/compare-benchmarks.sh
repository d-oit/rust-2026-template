#!/usr/bin/env bash
# scripts/compare-benchmarks.sh
# Compare benchmark results between two commits and report regressions.
# Usage: ./scripts/compare-benchmarks.sh [--commit-a <sha>] [--commit-b <sha>]
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT" || exit 1

# --- Colors ---
if [[ -t 1 ]] && [[ "${FORCE_COLOR:-}" != "0" ]]; then
  RED='\033[0;31m'
  YELLOW='\033[1;33m'
  NC='\033[0m'
else
  RED=''
  YELLOW=''
  NC=''
fi

# --- Parse arguments ---
COMMIT_A=""
COMMIT_B=""
for arg in "$@"; do
  case $arg in
    --commit-a) shift; COMMIT_A="${1:-}"; shift ;;
    --commit-a=*) COMMIT_A="${arg#*=}" ;;
    --commit-b) shift; COMMIT_B="${1:-}"; shift ;;
    --commit-b=*) COMMIT_B="${arg#*=}" ;;
    *) echo "Unknown argument: $arg"; exit 1 ;;
  esac
done

# Default: compare HEAD~1 vs HEAD
if [[ -z "$COMMIT_A" ]]; then
  COMMIT_A=$(git rev-parse HEAD~1 2>/dev/null || echo "")
fi
if [[ -z "$COMMIT_B" ]]; then
  COMMIT_B=$(git rev-parse HEAD 2>/dev/null || echo "")
fi

if [[ -z "$COMMIT_A" || -z "$COMMIT_B" ]]; then
  echo -e "${RED}[ERROR]${NC} Could not determine commits to compare"
  echo "Usage: ./scripts/compare-benchmarks.sh [--commit-a <sha>] [--commit-b <sha>]"
  exit 1
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Benchmark Comparison"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Baseline:  ${COMMIT_A:0:8}"
echo "  Current:   ${COMMIT_B:0:8}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Find event files for each commit
find_events() {
  local commit="$1"
  find benchmarks/events -name "${commit}*.jsonl" -type f 2>/dev/null | head -5
}

EVENTS_A=$(find_events "$COMMIT_A")
EVENTS_B=$(find_events "$COMMIT_B")

if [[ -z "$EVENTS_A" ]]; then
  echo -e "${YELLOW}[WARN]${NC} No benchmark events found for commit ${COMMIT_A:0:8}"
  echo "  Run 'cargo bench --workspace -- --output-format bencher' to generate baseline"
  exit 0
fi

if [[ -z "$EVENTS_B" ]]; then
  echo -e "${YELLOW}[WARN]${NC} No benchmark events found for commit ${COMMIT_B:0:8}"
  echo "  Run 'cargo bench --workspace -- --output-format bencher' to generate current results"
  exit 0
fi

# Parse and compare if python3 is available
if command -v python3 &>/dev/null; then
  python3 - "$EVENTS_A" "$EVENTS_B" << 'PYTHON_SCRIPT'
import json
import sys
from pathlib import Path

def load_events(files):
    benchmarks = {}
    for f in files:
        p = Path(f)
        if not p.exists():
            continue
        with open(p) as fh:
            for line in fh:
                line = line.strip()
                if not line:
                    continue
                try:
                    data = json.loads(line)
                    name = data.get('benchmark', 'unknown')
                    ns = data.get('nanoseconds', 0)
                    if ns > 0:
                        benchmarks[name] = ns
                except json.JSONDecodeError:
                    continue
    return benchmarks

events_a = sys.argv[1].split('\n') if sys.argv[1] else []
events_b = sys.argv[2].split('\n') if sys.argv[2] else []

benchmarks_a = load_events(events_a)
benchmarks_b = load_events(events_b)

if not benchmarks_a and not benchmarks_b:
    print("No benchmark data available for comparison")
    sys.exit(0)

# Compare
regressions = []
improvements = []
unchanged = []

for name in sorted(set(list(benchmarks_a.keys()) + list(benchmarks_b.keys()))):
    if name in benchmarks_a and name in benchmarks_b:
        time_a = benchmarks_a[name]
        time_b = benchmarks_b[name]
        change_pct = ((time_b - time_a) / time_a) * 100

        if change_pct > 10:
            regressions.append((name, time_a, time_b, change_pct))
        elif change_pct < -10:
            improvements.append((name, time_a, time_b, change_pct))
        else:
            unchanged.append((name, time_a, time_b, change_pct))
    elif name in benchmarks_b:
        improvements.append((name, 0, benchmarks_b[name], -100))

# Print results
if regressions:
    print("REGRESSIONS (>10% slower):")
    for name, time_a, time_b, pct in sorted(regressions, key=lambda x: -x[3]):
        print(f"  {name}: {time_a/1000:.1f}us -> {time_b/1000:.1f}us (+{pct:.1f}%)")
    print()

if improvements:
    print("IMPROVEMENTS (>10% faster):")
    for name, time_a, time_b, pct in sorted(improvements, key=lambda x: x[3]):
        if time_a > 0:
            print(f"  {name}: {time_a/1000:.1f}us -> {time_b/1000:.1f}us ({pct:.1f}%)")
        else:
            print(f"  {name}: new benchmark")
    print()

if unchanged:
    print(f"UNCHANGED: {len(unchanged)} benchmarks within 10%")

if not regressions and not improvements:
    print("No significant changes detected")

if regressions:
    sys.exit(1)
PYTHON_SCRIPT
else
  echo -e "${YELLOW}[WARN]${NC} python3 not found, cannot parse benchmark data"
  echo "  Install python3 to enable benchmark comparison"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
