---
name: metrics-reporter
description: "Record agentic task completion via event files for DORA observability."

license: MIT
metadata:
  category: metrics
  author: jules
  version: "2.0"
---

# Skill: metrics-reporter

## Purpose

Record agentic task completion by writing a per-event JSON file to `.agents/events/`. This pattern avoids Git merge conflicts and supports multi-agent coordination.

## When to use

Call this skill as the LAST step of any task or sub-task.

## Event schema

Schema of record: the Event File Format in [`.agents/ORCHESTRATION.md`](../ORCHESTRATION.md)
(gated by the `SkillValidation`/`SkillEvals` checks). CI validates a different,
legacy line format (`.agents/metrics.jsonl`, required fields `timestamp`,
`agent`, `skill`, `success` — see the "Validate agent metrics schema" step in
`.github/workflows/ci.yml`), but that file is legacy: `dora-report` reads it as
its `--agent-metrics` input and **no workflow appends to it anymore** — since
the cutover to event files it only holds its pre-cutover lines. New records go
to event files.

## Steps

1. **Prepare Directory**: Ensure the daily directory exists.

   ```bash
   EVENT_DIR=".agents/events/$(date -u +%Y/%m/%d)"
   mkdir -p "${EVENT_DIR}"
   ```

2. **Generate Filename**: Create a unique filename for the event.

   ```bash
   EVENT_FILE="${EVENT_DIR}/$(date -u +%Y-%m-%dT%H-%M-%SZ)-${AGENT_NAME:-unknown}-${TASK_ID:-$(date -u +%s)}.json"
   ```

3. **Write Event Data**: One immutable JSON document per file, per the
   `.agents/ORCHESTRATION.md` schema. Required fields:

   ```bash
   cat > "${EVENT_FILE}" <<EOF
   {
     "event_id": "$(date -u +%s)-${AGENT_NAME:-unknown}",
     "task_id": "${TASK_ID:-$(date -u +%s)}",
     "parent_task_id": ${PARENT_TASK_ID:-null},
     "workflow_id": ${WORKFLOW_ID:-null},
     "agent_id": "${AGENT_NAME:-unknown}",
     "agent_type": "${AGENT_TYPE:-unknown}",
     "skill": "${SKILL_NAME:-unknown}",
     "skill_version": "${SKILL_VERSION:-0.0.0}",
     "status": "${STATUS:-success}",
     "started_at": "${STARTED_AT:-$(date -u +%Y-%m-%dT%H:%M:%SZ)}",
     "finished_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
     "success": ${SUCCESS:-true},
     "human_interventions": ${HUMAN_INTERVENTIONS:-0},
     "git_branch": "$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)",
     "git_sha": "$(git rev-parse --short HEAD 2>/dev/null || echo unknown)",
     "pr_number": ${PR_NUMBER:-null},
     "artifacts": [],
     "notes": "${NOTES:-}"
   }
   EOF
   ```

   Field notes:
   - `agent_id` (NOT `agent`) — the aggregator groups by `agent_id` and
     unknown keys are silently bucketed as `"unknown"` by
     `scripts/aggregate-metrics.sh`.
   - `status`: `success` | `failure` | `partial`.
   - `success` is a real JSON boolean; `human_interventions` a real number (no
     quotes — quoted values break aggregation statistics).
   - Files are immutable once written: never edit or delete an event file; a
     retry writes a new file with a new timestamp prefix.

4. **Verify**: Validate the JSON and confirm the path:

   ```bash
   python3 -m json.tool "${EVENT_FILE}" > /dev/null && echo "Event OK: ${EVENT_FILE}"
   ```

## Stacked PR Metrics

When using the `stacked-prs` skill, add these optional fields to the event JSON:

```json
{
  "stack_id": "billing-feature",
  "stack_position": 1,
  "stack_total": 3
}
```

- `stack_id`: Identifier for the stack (feature name)
- `stack_position`: This layer's position in the stack (1-indexed)
- `stack_total`: Total number of layers in the stack

Do NOT include these fields for non-stacked work.

## Handoff Coordination

Per `.agents/ORCHESTRATION.md`, a skill that hands off to another skill must also update `.agents/context/workflow-state.json` (`current_phase`, `last result`, `pending_skill`) after writing its event file. Reading agents pick the state up from there.

## Human Interventions

Set `human_interventions > 0` if:
- A human corrected, rewrote, or reverted your code.
- A PR required a fixup commit after your submission.
- A review comment called out an error in your work.

## Aggregation

Event files are aggregated by `scripts/aggregate-metrics.sh` into
`.agents/aggregated/metrics.jsonl` and `.agents/aggregated/daily-summary.json`.
The script reads `agent_id`, `skill`, `finished_at`/`started_at`, `success`,
and `human_interventions` — hence the field requirements above. It is invoked
by CI after merges to `main`; it can also be run locally with
`bash scripts/aggregate-metrics.sh`. CI status files live in `.agents/ci/` and,
with `.agents/aggregated/`, are generated output: do not hand-edit them.

## Rationalizations

| Rationalization | Reality |
|-----------------|---------|
| "I'll skip metrics — it's just paperwork." | Without metrics, you can't prove AI tooling ROI or identify bottlenecks. |
| "I'll log it later." | Later never happens. Log immediately after task completion. |
| "human_interventions doesn't matter." | It's the key metric for measuring true agent autonomy vs human effort. |

## Red Flags

- [ ] Not calling metrics-reporter as the last step of a task
- [ ] Setting human_interventions to 0 when a human fixed your code
- [ ] Writing `"agent"` instead of `"agent_id"` (silently aggregated as unknown)
- [ ] Using the old append-to-metrics.jsonl pattern instead of event files