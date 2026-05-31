# Agent Orchestration

This document defines multi-agent coordination: skill chaining, agent roles, handoff protocols, and workflow pipelines.
**All agents must read this file before executing any skill that may be part of a larger workflow.**

## Agent Roles

| Role | Agent(s) | Owned Skills |
|------|----------|--------------|
| **code-agent** | Claude Code, Gemini CLI, Qwen Code | `build-rust`, `lint-rust`, `test-rust`, `anti-ai-slop`, `privacy-first` |
| **release-agent** | Claude Code, OpenCode | `release-rust`, `crates-io-name-check` |
| **quality-agent** | Any | `skill-evaluator`, `codacy`, `dora-report`, `metrics-reporter` |
| **meta-agent** | Any | `skill-creator`, `skill-evaluator` |

## Skill Dependency Graph

The following chains define valid ordered pipelines. A skill must not start until all prerequisite skills report `success: true`.

```
build-rust
  └── lint-rust
        └── test-rust
              └── release-rust
                    └── crates-io-name-check (pre-release only)

anti-ai-slop  ──┐
privacy-first ──┤──► lint-rust ──► test-rust
codacy        ──┘

skill-creator ──► skill-evaluator

dora-report       (runs independently on schedule, reads event files)
metrics-reporter  (runs after any skill completes)
```

## Handoff Protocol

When one agent hands off to another (or one skill triggers the next), it **must**:

1. Write a per-event file to `.agents/events/YYYY/MM/DD/` — see [Event File Format](#event-file-format).
2. Update `.agents/context/workflow-state.json` with the current phase, last result, and `pending_skill`.
3. The receiving agent reads `workflow-state.json` to understand current state before invoking its skill.
4. After completion, the receiving agent updates `workflow-state.json` and writes its own event file.

### Example workflow-state.json (after build-rust succeeds)

```jsonc
{
  "workflow_id": "wf-2026-05-31-001",
  "current_phase": "lint",
  "active_skill": "lint-rust",
  "last_build_status": "success",
  "pending_skill": "test-rust",
  "git_branch": "feat/my-feature",
  "git_sha": "abc1234",
  "completed_skills": ["build-rust"],
  "failed_skills": []
}
```

## Event File Format

Each agent task writes a **single immutable JSON file** — it never modifies an existing file.

### Path convention

```
.agents/events/YYYY/MM/DD/<ISO-timestamp>-<agent>-<task-id>.json
```

Because each file has a unique path (timestamp + agent + task-id), **two concurrent agents can never write to the same file**.

### Schema

```jsonc
{
  "event_id": "<timestamp>-<agent>",
  "task_id": "<unique task identifier>",
  "parent_task_id": "<id of triggering task, or null>",
  "workflow_id": "<workflow group identifier, or null>",
  "agent_id": "claude",
  "agent_type": "claude-code",
  "skill": "build-rust",
  "skill_version": "1.0.0",
  "status": "success",        // success | failure | partial
  "started_at": "2026-05-31T14:00:00Z",
  "finished_at": "2026-05-31T14:01:30Z",
  "success": true,
  "human_interventions": 0,
  "git_branch": "feat/my-feature",
  "git_sha": "abc1234",
  "pr_number": null,
  "artifacts": [],            // list of file paths or URLs produced
  "notes": ""
}
```

### Write command (bash)

```bash
# Required env vars: AGENT_NAME, TASK_ID, SKILL_NAME
# Optional:         WORKFLOW_ID, PARENT_TASK_ID, AGENT_TYPE, SKILL_VERSION,
#                   PR_NUMBER, STATUS, SUCCESS, HUMAN_INTERVENTIONS, NOTES

EVENT_DIR=".agents/events/$(date -u +%Y/%m/%d)"
mkdir -p "${EVENT_DIR}"
EVENT_FILE="${EVENT_DIR}/$(date -u +%Y-%m-%dT%H-%M-%SZ)-${AGENT_NAME:-unknown}-${TASK_ID:-$(date -u +%s)}.json"

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

echo "Event written: ${EVENT_FILE}"
```

## Conflict-Free Guarantee

- Each event file has a **unique path** (timestamp + agent + task-id). Two concurrent agents never write to the same file.
- `workflow-state.json` is updated sequentially within a single workflow. For independent parallel workflows, use separate workflow IDs.
- `.agents/aggregated/metrics.jsonl` is **generated output** — rebuilt by `scripts/aggregate-metrics.sh` in CI, never hand-edited.
- Event files under `.agents/events/` are **append-only immutable** — once written, never edited or deleted.

## Related Files

| File | Purpose |
|------|---------|
| [AGENTS.md](../AGENTS.md) | General agent guidelines |
| [.agents/SKILLS.md](SKILLS.md) | Skill index, format, and Input/Output schemas |
| [.agents/context/workflow-state.json](context/workflow-state.json) | Live workflow state (read before acting) |
| [scripts/aggregate-metrics.sh](../scripts/aggregate-metrics.sh) | Aggregation script (run in CI) |
