#!/usr/bin/env bash
# scripts/distill-strikes.sh — Scaffolds starter skill & red fixture from repeated strike signatures
# Usage: ./scripts/distill-strikes.sh [--threshold N] [--matrix PATH] [--run-history PATH] [--output-dir PATH] [--dry-run]
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

THRESHOLD=3
MATRIX_FILE=".agents/ci/regression-matrix.json"
RUN_HISTORY_FILE=".agents/ci/quality-run.json"
OUTPUT_DIR=".agents/skills/drafts"
SKILLS_DIR=".agents/skills"
DRY_RUN=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --threshold)
      THRESHOLD="$2"
      shift 2
      ;;
    --matrix)
      MATRIX_FILE="$2"
      shift 2
      ;;
    --run-history)
      RUN_HISTORY_FILE="$2"
      shift 2
      ;;
    --output-dir)
      OUTPUT_DIR="$2"
      shift 2
      ;;
    --skills-dir)
      SKILLS_DIR="$2"
      shift 2
      ;;
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    -h|--help)
      echo "Usage: $0 [--threshold N] [--matrix PATH] [--run-history PATH] [--output-dir PATH] [--skills-dir PATH] [--dry-run]"
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
  esac
done

if [[ ! -f "$MATRIX_FILE" ]]; then
  echo "Error: Matrix file $MATRIX_FILE not found." >&2
  exit 1
fi

python3 - "$MATRIX_FILE" "$RUN_HISTORY_FILE" "$THRESHOLD" "$OUTPUT_DIR" "$SKILLS_DIR" "$DRY_RUN" <<'EOF'
import json
import os
import sys

matrix_path = sys.argv[1]
run_history_path = sys.argv[2]
threshold = int(sys.argv[3])
output_dir = sys.argv[4]
skills_dir = sys.argv[5]
dry_run = sys.argv[6].lower() == 'true'

with open(matrix_path, 'r', encoding='utf-8') as f:
    matrix_data = json.load(f)

cases = matrix_data.get('cases', [])

# Cluster cases by subsystem (or failure_category if subsystem missing)
clusters = {}
for c in cases:
    subsystem = c.get('subsystem') or c.get('failure_category') or 'unknown'
    # Sanitize subsystem name for directory/skill name
    clean_subsystem = subsystem.lower().replace(' / ', '-').replace('/', '-').replace(' ', '-')
    clean_subsystem = ''.join(ch for ch in clean_subsystem if ch.isalnum() or ch == '-')

    if clean_subsystem not in clusters:
        clusters[clean_subsystem] = {
            'raw_subsystem': subsystem,
            'count': 0,
            'cases': []
        }
    clusters[clean_subsystem]['count'] += 1
    clusters[clean_subsystem]['cases'].append(c)

# Also check run_history if present
if os.path.isfile(run_history_path):
    try:
        with open(run_history_path, 'r', encoding='utf-8') as f:
            history_data = json.load(f)
            # If run_history contains failures or violations
            history_failures = history_data.get('failures', [])
            for h in history_failures:
                sub = h.get('subsystem') or h.get('sensor') or 'unknown'
                clean_sub = sub.lower().replace(' / ', '-').replace('/', '-').replace(' ', '-')
                clean_sub = ''.join(ch for ch in clean_sub if ch.isalnum() or ch == '-')
                if clean_sub in clusters:
                    clusters[clean_sub]['count'] += 1
                    clusters[clean_sub]['cases'].append(h)
    except Exception as e:
        print(f"Notice: Failed to parse run history {run_history_path}: {e}", file=sys.stderr)

scaffolded_count = 0
skipped_count = 0

for clean_sub, info in clusters.items():
    raw_sub = info['raw_subsystem']
    count = info['count']
    sample_case = info['cases'][0] if info['cases'] else {}

    if count < threshold:
        continue

    # Check if an existing skill already exists in skills_dir
    existing_skill_file = os.path.join(skills_dir, clean_sub, "SKILL.md")
    if os.path.exists(existing_skill_file) or os.path.exists(os.path.join(skills_dir, clean_sub)):
        print(f"REFUSED: Existing skill already exists at {os.path.join(skills_dir, clean_sub)}. Skipping scaffold for '{raw_sub}' (count: {count}).")
        skipped_count += 1
        continue

    # Prepare starter skill frontmatter & content
    skill_name = clean_sub
    summary = sample_case.get('summary', f"Repeated failures detected for {raw_sub}.")
    remediation_hint = sample_case.get('remediation_hint', sample_case.get('fix_hint', 'No remediation hint provided.'))
    repro_cmd = sample_case.get('reproduction_command', sample_case.get('command', 'N/A'))

    signatures = sample_case.get('detection_signatures', [])
    sig_str = "\n".join(f"- `{sig}`" for sig in signatures) if signatures else "- N/A"

    failing_cases_summary = []
    for case in info['cases']:
        case_id = case.get('id', 'N/A')
        case_workflow = case.get('workflow', case.get('sensor', 'N/A'))
        case_run = case.get('run_id', 'N/A')
        failing_cases_summary.append(f"- Case ID: `{case_id}` | Workflow/Sensor: `{case_workflow}` | Run ID: `{case_run}`")
    cases_block = "\n".join(failing_cases_summary)

    skill_md_content = f"""---
name: {skill_name}
description: >
  Starter guide distilled from repeated sensor strikes for {raw_sub}.
  Triggers: "{skill_name}", "{raw_sub} failure", "sensor strike".
category: rust
license: MIT
metadata:
  author: distill-strikes
  version: "0.1"
  subsystem: "{raw_sub}"
  strikes: {count}
---

# Skill: {skill_name}

## Summary

This starter skill was automatically scaffolded by `distill-strikes.sh` because the `{raw_sub}` sensor signature reached the fail-fast threshold ({count} occurrences >= threshold {threshold}).

## Failing Sensor Context & HARNESS VIOLATION Hint

### Detection Signatures
{sig_str}

### VERBATIM HARNESS VIOLATION / REMEDIATION HINT
> **{remediation_hint}**

### Reproduction Command
```bash
{repro_cmd}
```

### Strike History
{cases_block}

## Instructions

1. **Verify Root Cause**: Run the reproduction command locally and inspect the verbatim output.
2. **Apply Minimal Fix**: Address the violation without introducing unrelated refactoring.
3. **Verify Green Sensor**: Confirm that the sensor passes cleanly before proposing skill promotion.

## Rationalizations

| Rationalization | Reality |
|-----------------|---------|
| "We can ignore repeated sensor strikes." | Unhandled strike patterns cause recurrent CI failures and drain developer velocity. |
| "A draft skill can be promoted without green sensor verification." | A guide distilled from a fix must be verified against a green sensor run before promotion. |

## Red Flags

- [ ] Overwriting an existing skill when scaffolding from strikes
- [ ] Promoting a draft skill without verifying a green sensor run
- [ ] Removing failing test cases without updating feedforward guides
"""

    red_fixture_content = f"""# Red Fixture Sample for {raw_sub}
# Scaffolded by distill-strikes.sh on strike count: {count}

# Failing Detection Signatures:
{sig_str}

# Verbatim Remediation Hint:
# {remediation_hint}

# Reproduction Command:
# {repro_cmd}
"""

    draft_skill_dir = os.path.join(output_dir, clean_sub)
    draft_skill_file = os.path.join(draft_skill_dir, "SKILL.md")
    fixtures_dir = os.path.join(draft_skill_dir, "fixtures")
    red_fixture_file = os.path.join(fixtures_dir, "red_fixture.sample")

    if dry_run:
        print(f"[DRY-RUN] Would scaffold starter skill for '{raw_sub}' (strikes: {count}) at {draft_skill_file}")
        print(f"[DRY-RUN] Would scaffold red fixture at {red_fixture_file}")
    else:
        os.makedirs(fixtures_dir, exist_ok=True)
        with open(draft_skill_file, 'w', encoding='utf-8') as sf:
            sf.write(skill_md_content)
        with open(red_fixture_file, 'w', encoding='utf-8') as ff:
            ff.write(red_fixture_content)
        print(f"SCAFFOLDED: Draft skill created at {draft_skill_file} (strikes: {count})")
        print(f"SCAFFOLDED: Red fixture sample created at {red_fixture_file}")

    scaffolded_count += 1

print(f"Done. Scaffolded: {scaffolded_count}, Skipped (existing): {skipped_count}")
EOF
