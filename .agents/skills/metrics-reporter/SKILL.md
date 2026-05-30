---
name: metrics-reporter
description: "Append a DORA agentic harness metrics record to .agents/metrics.jsonl after completing any task."
license: MIT
metadata:
  author: jules
  version: "1.0"
---

# Skill: metrics-reporter

## Purpose

Append a DORA agentic harness metrics record to `.agents/metrics.jsonl` after completing any task.

## When to use

Call this skill as the LAST step of any other skill execution.

## Steps

1. **Collect task metadata**: agent name, skill used, PR number (if any), success status.
2. **Count human interventions**: from git log or review comments (0 if none).
3. **Estimate token usage**: if the runtime provides it.
4. **Append record**: to `.agents/metrics.jsonl` using the defined schema.
5. **Stage and commit**: `.agents/metrics.jsonl` with message: `chore(metrics): record agent task completion [skip ci]`.

## Schema Reference

See `.agents/metrics.jsonl` and `agents-docs/dora-metrics.md` for field definitions.

### Minimal append command (bash)

```bash
cat >> .agents/metrics.jsonl << EOF
{"timestamp":"$(date -u +%Y-%m-%dT%H:%M:%SZ)","agent":"jules","skill":"build-rust","task_description":"...","success":true,"human_interventions":0}
EOF
```
