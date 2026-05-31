#!/usr/bin/env bash
# aggregate-metrics.sh
#
# Aggregates all .agents/events/**/*.json files into:
#   .agents/aggregated/metrics.jsonl   (one JSON object per line, chronological)
#   .agents/aggregated/daily-summary.json
#
# This file is GENERATED — do not hand-edit the output files.
# Run in CI after merging to main, or locally with: bash scripts/aggregate-metrics.sh

set -euo pipefail

EVENTS_DIR=".agents/events"
AGGREGATED_DIR=".agents/aggregated"
export OUTPUT_FILE="${AGGREGATED_DIR}/metrics.jsonl"
export SUMMARY_FILE="${AGGREGATED_DIR}/daily-summary.json"

mkdir -p "${AGGREGATED_DIR}"

echo "[aggregate-metrics] Scanning ${EVENTS_DIR} for event files..."

# Reset output file (SC2188 fix: use `: >` instead of bare `>`)
: > "${OUTPUT_FILE}"

# Find all event JSON files, sort chronologically by path (YYYY/MM/DD/timestamp prefix)
EVENT_COUNT=0
INVALID_COUNT=0

while IFS= read -r -d '' f; do
  if python3 -c "import json,sys; json.load(open('${f}'))" 2>/dev/null; then
    python3 -c "import json,sys; print(json.dumps(json.load(open('${f}'))))" >> "${OUTPUT_FILE}"
    EVENT_COUNT=$((EVENT_COUNT + 1))
  else
    echo "[aggregate-metrics] WARNING: Skipping invalid JSON: ${f}" >&2
    INVALID_COUNT=$((INVALID_COUNT + 1))
  fi
done < <(find "${EVENTS_DIR}" -name '*.json' ! -name '.gitkeep' -print0 | sort -z)

echo "[aggregate-metrics] ${EVENT_COUNT} events aggregated, ${INVALID_COUNT} skipped."

# Build summary with Python (reads OUTPUT_FILE and SUMMARY_FILE from env)
python3 - <<'PYEOF'
import json, os
from datetime import datetime, timezone
from pathlib import Path

output_file = Path(os.environ.get('OUTPUT_FILE', '.agents/aggregated/metrics.jsonl'))
summary_file = Path(os.environ.get('SUMMARY_FILE', '.agents/aggregated/daily-summary.json'))

events = []
if output_file.exists():
    for line in output_file.read_text().splitlines():
        line = line.strip()
        if line:
            try:
                events.append(json.loads(line))
            except json.JSONDecodeError:
                pass

by_agent = {}
by_skill = {}
by_date = {}
for e in events:
    agent = e.get('agent_id', 'unknown')
    skill = e.get('skill', 'unknown')
    ts = e.get('finished_at', e.get('started_at', ''))
    date = ts[:10] if ts else 'unknown'
    by_agent[agent] = by_agent.get(agent, 0) + 1
    by_skill[skill] = by_skill.get(skill, 0) + 1
    by_date[date] = by_date.get(date, 0) + 1

summary = {
    'generated_at': datetime.now(timezone.utc).isoformat(),
    'total_events': len(events),
    'successes': sum(1 for e in events if e.get('success') is True),
    'failures': sum(1 for e in events if e.get('success') is False),
    'human_interventions_total': sum(e.get('human_interventions', 0) for e in events),
    'by_agent': dict(sorted(by_agent.items())),
    'by_skill': dict(sorted(by_skill.items())),
    'by_date': dict(sorted(by_date.items())),
}

with open(summary_file, 'w') as f:
    json.dump(summary, f, indent=2)

print(f'[aggregate-metrics] Summary written to {summary_file}')
print(f'[aggregate-metrics] Total: {summary["total_events"]} | Successes: {summary["successes"]} | Failures: {summary["failures"]}')
PYEOF
