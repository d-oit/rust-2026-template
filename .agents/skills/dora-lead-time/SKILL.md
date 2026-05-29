---
name: dora-lead-time
description: Use this skill to understand and measure Change Lead Time, a key DORA metric that tracks the time from PR creation to merge. This helps in assessing the impact of AI-assisted development on delivery speed.
license: MIT
metadata:
  author: d-oit
  version: "1.0"
---

# DORA Lead Time Skill

This skill provides guidance on how to interpret and utilize Change Lead Time data within this repository.

## What is Change Lead Time?

Change Lead Time is the duration from when a Pull Request is opened until it is merged into the `main` branch. It is a critical metric for measuring delivery throughput.

## Why We Measure It

- **Baseline Performance:** Establishing a historical baseline for how long changes take to deliver.
- **AI ROI:** Proving that agentic workflows and AI tools are actually reducing the time it takes to ship features and fixes.
- **Process Optimization:** Identifying bottlenecks in the PR review and CI/CD process.

## How to Access Lead Time Data

1. **GitHub Actions Summary:** Every merged PR to `main` will have a "DORA Change Lead Time" section in its action summary.
2. **Metrics Artifacts:** PR runs generate `dora-metrics.jsonl` artifacts.
3. **Historical Data:** Periodically, these metrics may be aggregated into a central `analysis/dora-metrics.jsonl` (manual or automated process).

## Agent Responsibilities

- **Self-Improvement:** When completing a task, agents should aim to be efficient without sacrificing quality, as their contribution to Lead Time is now visible.
- **Reporting:** When asked about project health or velocity, agents should refer to available DORA metrics.
- **Labeling:** Ensure PRs are labeled with `agentic` if they were primarily authored or assisted by an agent, as the workflow uses this label to categorize metrics.

## Example Metric Entry

```json
{"date":"2026-05-29","pr":95,"lead_time_hours":3.5,"author":"jules","agent_assisted":true}
```
