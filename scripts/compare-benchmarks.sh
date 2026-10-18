#!/usr/bin/env bash
# scripts/compare-benchmarks.sh
# Compare benchmark results between two runs with statistical noise boundaries.
#
# Supports strict regression gating on named reference runners and advisory
# evaluations for developer laptops / uncontrolled environments.
#
# Usage:
#   ./scripts/compare-benchmarks.sh [--commit-a <sha>|auto] [--commit-b <sha>]
#   ./scripts/compare-benchmarks.sh --baseline <file.jsonl> --current <file.jsonl>
#   ./scripts/compare-benchmarks.sh --mode <strict|advisory|auto> --tolerance-pct 5.0
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT" || exit 1

# --- Colors ---
if [[ -t 1 ]] && [[ "${FORCE_COLOR:-}" != "0" ]]; then
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  YELLOW='\033[1;33m'
  NC='\033[0m'
else
  RED=''
  GREEN=''
  YELLOW=''
  NC=''
fi

COMMIT_A=""
COMMIT_B=""
BASELINE_FILE=""
CURRENT_FILE=""
REFERENCE_RUNNER="${BENCH_REFERENCE_RUNNER:-}"
TOLERANCE_PCT="${BENCH_TOLERANCE_PCT:-5.0}"
MODE="${BENCH_MODE:-auto}"
UPDATE_BASELINE_FILE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --commit-a) COMMIT_A="${2:-}"; shift 2 ;;
    --commit-a=*) COMMIT_A="${1#*=}"; shift ;;
    --commit-b) COMMIT_B="${2:-}"; shift 2 ;;
    --commit-b=*) COMMIT_B="${1#*=}"; shift ;;
    --baseline) BASELINE_FILE="${2:-}"; shift 2 ;;
    --baseline=*) BASELINE_FILE="${1#*=}"; shift ;;
    --current) CURRENT_FILE="${2:-}"; shift 2 ;;
    --current=*) CURRENT_FILE="${1#*=}"; shift ;;
    --reference-runner) REFERENCE_RUNNER="${2:-}"; shift 2 ;;
    --reference-runner=*) REFERENCE_RUNNER="${1#*=}"; shift ;;
    --tolerance-pct) TOLERANCE_PCT="${2:-}"; shift 2 ;;
    --tolerance-pct=*) TOLERANCE_PCT="${1#*=}"; shift ;;
    --mode) MODE="${2:-}"; shift 2 ;;
    --mode=*) MODE="${1#*=}"; shift ;;
    --update-baseline) UPDATE_BASELINE_FILE="${2:-}"; shift 2 ;;
    --update-baseline=*) UPDATE_BASELINE_FILE="${1#*=}"; shift ;;
    *) echo "Unknown argument: $1"; exit 1 ;;
  esac
done

find_events() {
  local commit="$1"
  find benchmarks/events -name "${commit}*.jsonl" -type f 2>/dev/null | head -5
}

# Resolve baseline and current files
if [[ -n "$BASELINE_FILE" && -n "$CURRENT_FILE" ]]; then
  EVENTS_A="$BASELINE_FILE"
  EVENTS_B="$CURRENT_FILE"
else
  if [[ -z "$COMMIT_B" ]]; then
    COMMIT_B=$(git rev-parse HEAD 2>/dev/null || echo "")
  fi

  if [[ -z "$COMMIT_A" ]]; then
    COMMIT_A=$(git rev-parse HEAD~1 2>/dev/null || echo "")
  fi

  if [[ "${COMMIT_A}" == "auto" ]]; then
    COMMIT_A=""
    while IFS= read -r candidate; do
      sha="$(basename "${candidate}" .jsonl)"
      [[ "${sha}" == "${COMMIT_B}" ]] && continue
      if [[ -s "${candidate}" ]]; then
        COMMIT_A="${sha}"
        break
      fi
    done < <(find benchmarks/events -name '*.jsonl' -type f -printf '%T@ %p\n' 2>/dev/null \
      | sort -rn | awk '{print $2}')

    if [[ -z "${COMMIT_A}" ]]; then
      echo -e "${YELLOW}[WARN]${NC} No previous event-bearing benchmark run found"
      exit 0
    fi
  fi

  if [[ -z "$COMMIT_A" || -z "$COMMIT_B" ]]; then
    echo -e "${RED}[ERROR]${NC} Could not determine commits to compare"
    echo "Usage: ./scripts/compare-benchmarks.sh [--commit-a <sha>|auto] [--commit-b <sha>]"
    exit 1
  fi

  EVENTS_A=$(find_events "$COMMIT_A")
  EVENTS_B=$(find_events "$COMMIT_B")
fi

if [[ -z "$EVENTS_A" ]]; then
  echo -e "${YELLOW}[WARN]${NC} Baseline benchmark events not found"
  exit 0
fi

if [[ -z "$EVENTS_B" ]]; then
  echo -e "${YELLOW}[WARN]${NC} Current benchmark events not found"
  exit 0
fi

# If requested, update baseline artifact file
if [[ -n "$UPDATE_BASELINE_FILE" ]]; then
  mkdir -p "$(dirname "$UPDATE_BASELINE_FILE")"
  while IFS= read -r event_file; do
    [[ -n "$event_file" && -f "$event_file" ]] && cat "$event_file"
  done <<< "$EVENTS_B" > "$UPDATE_BASELINE_FILE"
  echo -e "${GREEN}[BASELINE UPDATED]${NC} Saved current run to $UPDATE_BASELINE_FILE"
fi

if ! command -v python3 &>/dev/null; then
  echo -e "${YELLOW}[WARN]${NC} python3 not found, skipping benchmark evaluation"
  exit 0
fi

python3 - "$EVENTS_A" "$EVENTS_B" "$MODE" "$REFERENCE_RUNNER" "$TOLERANCE_PCT" << 'PYTHON_SCRIPT'
import json
import sys
from pathlib import Path

def load_events(file_inputs):
    records = {}
    files = []
    for entry in file_inputs:
        for f in entry.split():
            if f.strip():
                files.append(Path(f.strip()))

    for p in files:
        if not p.exists():
            continue
        with open(p, encoding="utf-8") as fh:
            for line in fh:
                line = line.strip()
                if not line:
                    continue
                try:
                    data = json.loads(line)
                    name = data.get('benchmark', 'unknown')
                    ns = data.get('median_ns') or data.get('ns_per_iter') or data.get('nanoseconds') or 0
                    if ns <= 0:
                        continue

                    ci = data.get('confidence_interval') or {}
                    records[name] = {
                        "benchmark": name,
                        "median_ns": float(ns),
                        "lower_bound_ns": float(ci.get("lower_bound_ns", ns)),
                        "upper_bound_ns": float(ci.get("upper_bound_ns", ns)),
                        "confidence_level": float(ci.get("confidence_level", 0.95)),
                        "runner": data.get("runner", "unknown"),
                        "cpu": data.get("cpu", "unknown"),
                        "toolchain": data.get("toolchain", "unknown"),
                        "os": data.get("os", "unknown"),
                        "commit": data.get("commit", "unknown"),
                    }
                except (json.JSONDecodeError, ValueError):
                    continue
    return records

events_a_arg = sys.argv[1]
events_b_arg = sys.argv[2]
mode_arg = sys.argv[3]
ref_runner_arg = sys.argv[4]
tolerance_pct = float(sys.argv[5])

benchmarks_a = load_events([events_a_arg])
benchmarks_b = load_events([events_b_arg])

if not benchmarks_a or not benchmarks_b:
    print(f"No comparable data: baseline has {len(benchmarks_a)} benchmarks, current has {len(benchmarks_b)}")
    sys.exit(0)

# Environment Metadata summary
sample_b = next(iter(benchmarks_b.values()))
sample_a = next(iter(benchmarks_a.values()))

current_runner = sample_b.get("runner", "unknown")
current_commit = sample_b.get("commit", "unknown")
baseline_commit = sample_a.get("commit", "unknown")

# Resolve mode (strict vs advisory)
mode = mode_arg.lower()
if mode == "auto":
    if ref_runner_arg and current_runner == ref_runner_arg:
        mode = "strict"
    elif ref_runner_arg and ref_runner_arg in current_runner:
        mode = "strict"
    else:
        mode = "advisory"

print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
print("  Benchmark Comparison & Uncertainty Analysis")
print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
print(f"  Baseline:  {baseline_commit[:8]} | Runner: {sample_a.get('runner')} | CPU: {sample_a.get('cpu')}")
print(f"  Current:   {current_commit[:8]} | Runner: {current_runner} | CPU: {sample_b.get('cpu')}")
print(f"  Toolchain: {sample_b.get('toolchain')}")
print(f"  Tolerance: {tolerance_pct}% | Mode: {mode.upper()}")
print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
print()

regressions = []
inconclusive = []
improvements = []
unchanged = []

tolerance_factor = 1.0 + (tolerance_pct / 100.0)

all_names = sorted(set(list(benchmarks_a.keys()) + list(benchmarks_b.keys())))

for name in all_names:
    if name in benchmarks_a and name in benchmarks_b:
        rec_a = benchmarks_a[name]
        rec_b = benchmarks_b[name]

        med_a = rec_a["median_ns"]
        med_b = rec_b["median_ns"]

        lower_b = rec_b["lower_bound_ns"]
        upper_a = rec_a["upper_bound_ns"]

        pct_change = ((med_b - med_a) / med_a) * 100.0
        allowed_threshold = upper_a * tolerance_factor

        if med_b > (med_a * tolerance_factor):
            # Point estimate regressed beyond tolerance. Is it statistically significant?
            if lower_b > allowed_threshold:
                regressions.append((name, rec_a, rec_b, pct_change, allowed_threshold))
            else:
                inconclusive.append((name, rec_a, rec_b, pct_change, allowed_threshold))
        elif med_b < (med_a / tolerance_factor):
            improvements.append((name, rec_a, rec_b, pct_change))
        else:
            unchanged.append((name, rec_a, rec_b, pct_change))
    elif name in benchmarks_b:
        improvements.append((name, {}, benchmarks_b[name], 0.0))

if regressions:
    print("REGRESSIONS (Statistically significant > tolerance):")
    for name, rec_a, rec_b, pct, threshold in sorted(regressions, key=lambda x: -x[3]):
        med_a, med_b = rec_a["median_ns"], rec_b["median_ns"]
        lb_b, ub_b = rec_b["lower_bound_ns"], rec_b["upper_bound_ns"]
        print(f"  ❌ {name}: {med_a/1000:.2f}us -> {med_b/1000:.2f}us (+{pct:.1f}%) "
              f"[95% CI: {lb_b/1000:.2f}us - {ub_b/1000:.2f}us, Threshold: {threshold/1000:.2f}us]")
    print()

if inconclusive:
    print("STATISTICALLY INCONCLUSIVE / MEASUREMENT NOISE:")
    for name, rec_a, rec_b, pct, threshold in sorted(inconclusive, key=lambda x: -x[3]):
        med_a, med_b = rec_a["median_ns"], rec_b["median_ns"]
        lb_b, ub_b = rec_b["lower_bound_ns"], rec_b["upper_bound_ns"]
        print(f"  ⚠️  {name}: {med_a/1000:.2f}us -> {med_b/1000:.2f}us (+{pct:.1f}%) "
              f"[95% CI: {lb_b/1000:.2f}us - {ub_b/1000:.2f}us overlaps threshold {threshold/1000:.2f}us]")
    print()

if improvements:
    print("IMPROVEMENTS:")
    for name, rec_a, rec_b, pct in sorted(improvements, key=lambda x: x[3]):
        if rec_a:
            med_a, med_b = rec_a["median_ns"], rec_b["median_ns"]
            print(f"  🚀 {name}: {med_a/1000:.2f}us -> {med_b/1000:.2f}us ({pct:.1f}%)")
        else:
            print(f"  🚀 {name}: new benchmark baseline")
    print()

if unchanged:
    print(f"UNCHANGED / WITHIN TOLERANCE: {len(unchanged)} benchmark(s) within {tolerance_pct}%")
    print()

if mode == "strict":
    if regressions:
        print(f"❌ [STRICT MODE FAIL] Detected {len(regressions)} statistically significant regression(s) exceeding {tolerance_pct}% tolerance.")
        sys.exit(1)
    else:
        print(f"✓ [STRICT MODE PASS] No statistically significant regressions exceeding {tolerance_pct}% tolerance.")
        sys.exit(0)
else:
    print(f"ℹ️  [ADVISORY MODE] Comparison is informational for developer/uncontrolled environments ({current_runner}).")
    print("    No hard performance guarantee is enforced in advisory mode.")
    sys.exit(0)
PYTHON_SCRIPT
