---
# DORA Metrics

This repository tracks key DevOps Research and Assessment (DORA) metrics to
measure software delivery performance and stability.

## Change Lead Time

**Definition:** The time it takes for a commit to get into production.
**Tracking:** Measured from PR creation to merge into the `main` branch.
**Workflow:** `.github/workflows/dora-report.yml`

## Change Failure Rate (CFR)

**Definition:** The percentage of deployments or releases that cause a failure
in production and require immediate remediation (e.g., hotfix, rollback, or
patch).

### How We Track CFR

1. **Hotfix Detection:**
   - PRs labeled with `hotfix` or `rework` are tracked as remediation events.
   - Branches prefixed with `hotfix/` trigger automatic tracking.
   - The `.github/workflows/hotfix.yml` workflow records these events.

2. **Hotfix Releases:**
   - Releases triggered from hotfix branches or containing "hotfix",
     "regression", or "critical" in the last commit message are marked as
     hotfix releases.
   - These releases are annotated in GitHub Release notes.

3. **Metrics Storage:**
   - Events are appended to `dora-metrics.jsonl` and uploaded as workflow
     artifacts.
   - Each record includes the date, metric type (`deployment` or
     `change_failure`), and relevant metadata.

### Calculation

**CFR = (Total Hotfix Releases) / (Total Releases)**

A "Hotfix Release" is any release marked with `metric: change_failure` and
`type: hotfix` in the metrics log.

## Agentic Metrics

**Definition:** Measurement of AI agent performance, ROI, and human collaboration.
**Tracking:** Logged to `.agents/metrics.jsonl` by agents upon task completion.
**Workflow:** `.agents/skills/metrics-reporter/`

### Schema

Each record is a single-line JSON object (NDJSON):

```json
{
  "timestamp": "2026-05-29T20:00:00Z",
  "agent": "jules",
  "skill": "build-rust",
  "task_description": "Fix clippy warning in src/lib.rs",
  "pr_number": 95,
  "success": true,
  "human_interventions": 0,
  "tokens_used": 4200,
  "duration_seconds": 45,
  "code_reached_production": true,
  "notes": ""
}
```

### Field Definitions

| Field | Type | Description |
|---|---|---|
| `timestamp` | ISO 8601 | When the task completed |
| `agent` | string | Agent identifier (claude, gemini, qwen, jules, opencode) |
| `skill` | string | Skill from `.agents/skills/` that was used |
| `task_description` | string | Short description of the task |
| `pr_number` | int \| null | Associated PR number if applicable |
| `success` | bool | Did the agent complete the task without human override? |
| `human_interventions` | int | Number of times a human corrected the agent output |
| `tokens_used` | int \| null | Token count if available from the agent |
| `duration_seconds` | int \| null | Task execution time |
| `code_reached_production` | bool \| null | Whether the resulting code was merged to main |
| `notes` | string | Free-form notes (e.g., reason for failure) |

## Measurement and AI Impact

Following the **2025 DORA AI Report**, we actively monitor these metrics to
ensure that AI-assisted contributions (labeled `agentic`) maintain or improve
stability while increasing velocity. High velocity (low Lead Time) must be
balanced with low instability (low CFR).
