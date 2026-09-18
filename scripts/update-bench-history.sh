#!/usr/bin/env bash
# scripts/update-bench-history.sh
# Aggregates benchmarks/events/**/*.jsonl into benchmarks/history.jsonl.
#
# Events are authoritative for the (commit, benchmark) pairs they contain; rows already in
# history are preserved so the trend survives event retention pruning. Run this after the
# benchmark event files are written (the Benchmarks job does), then commit both.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

python3 - <<'PY'
import json
from pathlib import Path

root = Path("benchmarks")
history = root / "history.jsonl"


def load_rows(paths):
    rows = {}
    for path in paths:
        for line in path.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if not line:
                continue
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                continue
            key = (row.get("commit"), row.get("benchmark"))
            if None in key:
                continue
            rows[key] = row
    return rows


preserved = load_rows([history]) if history.exists() else {}
event_files = sorted(root.glob("events/**/*.jsonl"))
from_events = load_rows(event_files)

merged = {**preserved, **from_events}
ordered = sorted(merged.values(), key=lambda r: (r.get("timestamp") or "", r.get("benchmark") or ""))

history.write_text("".join(json.dumps(row, sort_keys=True) + "\n" for row in ordered), encoding="utf-8")
print(f"history.jsonl: {len(ordered)} rows ({len(from_events)} from {len(event_files)} event files, "
      f"{len(preserved)} preserved)")
PY
