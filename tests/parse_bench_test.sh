#!/usr/bin/env bash
# tests/parse_bench_test.sh
# Regression test for scripts/parse_bench.py: criterion estimates and harness-emitted
# bencher rows are both recorded, bencher rows win a name collision, and a missing
# criterion tree degrades gracefully instead of failing the bench job.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

CRITERION_DIR="${TMP_DIR}/criterion"
mkdir -p "${CRITERION_DIR}/tokio_cpu/single_unit_direct/new"
mkdir -p "${CRITERION_DIR}/tokio_fanout/bounded_semaphore_64/new"
printf '%s\n' '{"mean": {"point_estimate": 111.0}, "median": {"point_estimate": 41000.0}}' \
  > "${CRITERION_DIR}/tokio_cpu/single_unit_direct/new/estimates.json"
printf '%s\n' '{"median": {"point_estimate": 12096.4}}' \
  > "${CRITERION_DIR}/tokio_fanout/bounded_semaphore_64/new/estimates.json"

cat > "${TMP_DIR}/bench-output.txt" <<'TXT'
bench:  46,993 ns/iter (+/- 348)
test tokio_cpu/single_unit_direct/p50 ... bench: 40790 ns/iter (+/- 0)
test tokio_cpu/single_unit_direct ... bench: 99999 ns/iter (+/- 0)
TXT

python3 scripts/parse_bench.py "${TMP_DIR}/bench-output.txt" testsha 2026-09-17T00:00:00Z \
  "${CRITERION_DIR}" > "${TMP_DIR}/rows.jsonl"

python3 - "${TMP_DIR}/rows.jsonl" <<'PY'
import json
import sys

rows = {
    json.loads(line)["benchmark"]: json.loads(line)["ns_per_iter"]
    for line in open(sys.argv[1])
    if line.strip()
}
assert rows.get("tokio_cpu/single_unit_direct/p50") == 40790, "harness percentile row missing"
assert rows.get("tokio_fanout/bounded_semaphore_64") == 12096, "criterion estimate row missing"
assert rows.get("tokio_cpu/single_unit_direct") == 99999, "bencher row must win a name collision"
print(f"✓ merged {len(rows)} rows from both sources")
PY

# A missing criterion tree must degrade to the bencher rows instead of failing.
python3 scripts/parse_bench.py "${TMP_DIR}/bench-output.txt" testsha 2026-09-17T00:00:00Z \
  "${TMP_DIR}/absent" > "${TMP_DIR}/rows_without_criterion.jsonl"

python3 - "${TMP_DIR}/rows_without_criterion.jsonl" <<'PY'
import json
import sys

names = [json.loads(line)["benchmark"] for line in open(sys.argv[1]) if line.strip()]
assert names == ["tokio_cpu/single_unit_direct", "tokio_cpu/single_unit_direct/p50"], names
print("✓ missing criterion tree degrades to bencher rows only")
PY

echo "=== parse_bench tests PASSED ==="
