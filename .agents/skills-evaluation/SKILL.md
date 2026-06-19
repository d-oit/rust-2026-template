---
name: skills-evaluation
description: >
  Benchmarking and quality measurement framework for agent skills.
  Use when you need to measure skill effectiveness, run structured eval suites,
  or compare skill performance with/without specific enhancements.
  Triggers: "evaluate skills", "benchmark quality", "skill report".
category: meta
license: MIT
metadata:
  author: d-oit
  version: "1.0"
  source: d-o-hub/github-template-ai-agents
---

# Skill: skills-evaluation

## Purpose

Measure and improve the quality of agent skills through structured evaluation.

## When to Use

- When a new skill is added to ensure it meets structural requirements.
- When existing skills are updated to prevent regressions.
- When benchmarking the impact of skill descriptions or examples on agent performance.

## Evaluation Workflow

### 1. Structure Check

Run the automated structure check to ensure `SKILL.md` and `evals/evals.json` are present and valid.

```bash
./.agents/skills-evaluation/scripts/structure_check.sh
```

### 2. Live Evaluation

Use the provided workspaces in `.agents/skills-evaluation/workspaces/` to run agents with and without specific skills.

- **with_skill**: Agent has access to the skill definition.
- **without_skill**: Agent performs the task using only general knowledge.

### 3. Report Generation

Aggregate results into a human-readable report.

```bash
python3 .agents/skills-evaluation/scripts/generate_report.py
```

## Scoring Criteria

| Score | Meaning |
|-------|---------|
| 8/8   | **PASS**: Perfect structure, complete evals and assertions. |
| 5-7/8 | **NEEDS_WORK**: Usable, but missing some meta-data or enough eval cases. |
| <5/8  | **FAIL**: Missing core components like SKILL.md or major sections. |

## Rationalizations

| Rationalization | Reality |
|-----------------|---------|
| "The skill works in my manual test." | Manual tests are not repeatable. Automated structure and eval checks prevent drift. |
| "Writing evals takes too much time." | Evals are documentation of what "good" looks like. They save time during review. |

## Red Flags

- [ ] Skill missing `Rationalizations` section (hides design trade-offs).
- [ ] Less than 3 eval cases in `evals.json` (insufficient coverage).
- [ ] Subjective assertions in evals (cannot be objectively graded).

## Related Skills

- `skill-creator` - For building new skills that meet these standards.
- `skill-evaluator` - Reusable skill for performing the evaluation logic.
