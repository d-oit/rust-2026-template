#!/usr/bin/env bash
# tests/compare_benchmarks_test.sh
# Test suite for reference-runner benchmark comparison and uncertainty evaluation.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

BASELINE_FILE="${TMP_DIR}/baseline.jsonl"
REGRESSION_FILE="${TMP_DIR}/regression.jsonl"
WITHIN_TOLERANCE_FILE="${TMP_DIR}/within_tolerance.jsonl"
INCONCLUSIVE_FILE="${TMP_DIR}/inconclusive.jsonl"

# Baseline JSONL with metadata and confidence interval
cat > "$BASELINE_FILE" <<'EOF'
{"timestamp":"2026-10-17T00:00:00Z","commit":"111111111111","benchmark":"bench_a","ns_per_iter":100,"median_ns":100.0,"confidence_interval":{"lower_bound_ns":98.0,"upper_bound_ns":102.0,"confidence_level":0.95},"runner":"perf-runner-1","cpu":"AMD EPYC 7763","toolchain":"rustc 1.88.0","os":"Linux 6.6"}
EOF

# Candidate with definite regression (>5% tolerance and L_B > allowed threshold)
cat > "$REGRESSION_FILE" <<'EOF'
{"timestamp":"2026-10-17T01:00:00Z","commit":"222222222222","benchmark":"bench_a","ns_per_iter":120,"median_ns":120.0,"confidence_interval":{"lower_bound_ns":118.0,"upper_bound_ns":122.0,"confidence_level":0.95},"runner":"perf-runner-1","cpu":"AMD EPYC 7763","toolchain":"rustc 1.88.0","os":"Linux 6.6"}
EOF

# Candidate within tolerance (103ns vs 100ns baseline, 5% tolerance)
cat > "$WITHIN_TOLERANCE_FILE" <<'EOF'
{"timestamp":"2026-10-17T01:00:00Z","commit":"333333333333","benchmark":"bench_a","ns_per_iter":103,"median_ns":103.0,"confidence_interval":{"lower_bound_ns":101.0,"upper_bound_ns":105.0,"confidence_level":0.95},"runner":"perf-runner-1","cpu":"AMD EPYC 7763","toolchain":"rustc 1.88.0","os":"Linux 6.6"}
EOF

# Candidate statistically inconclusive / wide CI overlapping threshold
cat > "$INCONCLUSIVE_FILE" <<'EOF'
{"timestamp":"2026-10-17T01:00:00Z","commit":"444444444444","benchmark":"bench_a","ns_per_iter":108,"median_ns":108.0,"confidence_interval":{"lower_bound_ns":95.0,"upper_bound_ns":121.0,"confidence_level":0.95},"runner":"perf-runner-1","cpu":"AMD EPYC 7763","toolchain":"rustc 1.88.0","os":"Linux 6.6"}
EOF

echo "=== Test 1: Synthetic regression above tolerance fails strict mode ==="
set +e
OUT=$(bash scripts/compare-benchmarks.sh --baseline "$BASELINE_FILE" --current "$REGRESSION_FILE" --mode strict --tolerance-pct 5.0 2>&1)
STATUS=$?
set -e
if [[ $STATUS -eq 0 ]]; then
  echo "FAILED: Strict comparison should have failed on synthetic regression"
  echo "$OUT"
  exit 1
fi
if ! echo "$OUT" | grep -q "STRICT MODE FAIL"; then
  echo "FAILED: Expected STRICT MODE FAIL message in output"
  echo "$OUT"
  exit 1
fi
echo "✓ Strict mode failed on synthetic regression as expected"

echo "=== Test 2: Synthetic regression passes in advisory mode ==="
set +e
OUT=$(bash scripts/compare-benchmarks.sh --baseline "$BASELINE_FILE" --current "$REGRESSION_FILE" --mode advisory --tolerance-pct 5.0 2>&1)
STATUS=$?
set -e
if [[ $STATUS -ne 0 ]]; then
  echo "FAILED: Advisory comparison should never exit non-zero"
  echo "$OUT"
  exit 1
fi
if ! echo "$OUT" | grep -q "ADVISORY MODE"; then
  echo "FAILED: Expected ADVISORY MODE notice in output"
  echo "$OUT"
  exit 1
fi
echo "✓ Advisory mode passed on synthetic regression with advisory notice"

echo "=== Test 3: Change within tolerance passes strict mode ==="
OUT=$(bash scripts/compare-benchmarks.sh --baseline "$BASELINE_FILE" --current "$WITHIN_TOLERANCE_FILE" --mode strict --tolerance-pct 5.0)
if ! echo "$OUT" | grep -q "STRICT MODE PASS"; then
  echo "FAILED: Expected STRICT MODE PASS for change within tolerance"
  echo "$OUT"
  exit 1
fi
echo "✓ Change within tolerance passed strict mode"

echo "=== Test 4: Statistically inconclusive change passes strict mode ==="
OUT=$(bash scripts/compare-benchmarks.sh --baseline "$BASELINE_FILE" --current "$INCONCLUSIVE_FILE" --mode strict --tolerance-pct 5.0)
if ! echo "$OUT" | grep -q "STRICT MODE PASS"; then
  echo "FAILED: Statistically inconclusive change should not fail strict mode"
  echo "$OUT"
  exit 1
fi
if ! echo "$OUT" | grep -q "INCONCLUSIVE"; then
  echo "FAILED: Expected INCONCLUSIVE / MEASUREMENT NOISE status"
  echo "$OUT"
  exit 1
fi
echo "✓ Statistically inconclusive change passed strict mode without false failure"

echo "=== Test 5: Baseline update saves current run ==="
REF_BASELINE="${TMP_DIR}/updated_ref_baseline.jsonl"
bash scripts/compare-benchmarks.sh --baseline "$BASELINE_FILE" --current "$WITHIN_TOLERANCE_FILE" --update-baseline "$REF_BASELINE" >/dev/null
if [[ ! -f "$REF_BASELINE" ]]; then
  echo "FAILED: Updated reference baseline file was not created"
  exit 1
fi
if ! grep -q "bench_a" "$REF_BASELINE"; then
  echo "FAILED: Saved baseline missing expected benchmark data"
  exit 1
fi
echo "✓ Reference baseline update succeeded"

echo "=== Test 6: Auto mode selection with reference runner ==="
OUT_MATCH=$(bash scripts/compare-benchmarks.sh --baseline "$BASELINE_FILE" --current "$WITHIN_TOLERANCE_FILE" --mode auto --reference-runner "perf-runner-1")
if ! echo "$OUT_MATCH" | grep -q "STRICT"; then
  echo "FAILED: Matching reference runner should resolve auto mode to STRICT"
  echo "$OUT_MATCH"
  exit 1
fi

OUT_DEV=$(bash scripts/compare-benchmarks.sh --baseline "$BASELINE_FILE" --current "$WITHIN_TOLERANCE_FILE" --mode auto --reference-runner "other-runner-99")
if ! echo "$OUT_DEV" | grep -q "ADVISORY"; then
  echo "FAILED: Non-matching reference runner should resolve auto mode to ADVISORY"
  echo "$OUT_DEV"
  exit 1
fi
echo "✓ Auto mode correctly selected strict vs advisory based on reference runner"

echo "=== All compare_benchmarks tests PASSED ==="
