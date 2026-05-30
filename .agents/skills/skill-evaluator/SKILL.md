---
name: skill-evaluator
description: "Reusable skill for evaluating other skills with structure checks, eval coverage review, and real usage spot checks. Use when you need to check a skill, add evals, benchmark a skill, validate outputs against assertions, or compare current skill behavior against a baseline."
license: MIT
metadata:
  author: d-oit
  version: "1.1"
  source: d-o-hub/github-template-ai-agents
  spec: "agentskills.io"
---

# Skill Evaluator

Evaluate local skills with a repeatable loop: inspect structure, read eval definitions,
run one or more realistic prompts, then score the output with explicit assertions and evidence.

## When To Use

- Test whether a skill is wired correctly
- Check whether `evals/evals.json` exists and is usable
- Run a real prompt through a skill and grade the result
- Compare a skill against a no-skill baseline or older snapshot
- Identify missing folders, weak evals, and flaky assertions

## Required Inputs

At minimum, identify:

```text
SKILL_PATH: absolute or workspace-relative path to the skill directory
GOAL: structure check / eval review / live run / baseline comparison
```

## Evaluation Workflow

### 1. Structure Check

Confirm the skill directory is sane before judging outputs.

Expected layout:

```text
skill-name/
  SKILL.md
  evals/evals.json  # recommended
  references/       # recommended
  scripts/          # optional but useful
```

Flag these issues explicitly:

- missing `SKILL.md`
- nested duplicate directory like `skill-name/skill-name/`
- `evals/` exists but `evals/evals.json` is missing or invalid JSON
- eval cases missing `id`, `prompt`, or `expected_output`

### 2. Quantitative Evaluation

Consult `.agents/metrics.jsonl` to assess skill performance over time.
Analyze trends in:
- Success rate
- Human intervention rate
- Token usage and duration

### 3. Eval Review

Read `evals/evals.json` if present and assess whether each case is realistic.

Good evals include:

- a real user prompt
- a short success definition
- optional input files
- assertions that are concrete and checkable

Weak evals include:

- vague prompts
- purely subjective assertions
- no evidence path for pass/fail

### 4. Live Run

Run at least one representative prompt from the eval set or create a focused ad hoc prompt.

For each live run:

- load the target skill
- read only the files the skill itself points to
- produce the answer or output
- grade against assertions with evidence

### 5. Baseline Comparison

When useful, rerun the same prompt without the skill or against a snapshot of the older skill.

Compare:

- pass rate
- missing details
- format compliance
- time or token cost if available

### 6. Verdict

End with one of:

- `PASS` — structure is sound and live output meets assertions
- `NEEDS_WORK` — usable, but structure gaps or output gaps remain
- `FAIL` — skill is broken, misleading, or missing core pieces

## Assertion Rules

Prefer assertions that can be checked directly.

Good:

- `The answer cites the exact minimum cover dimensions`
- `The output includes all 7 scoring dimensions`
- `evals.json contains at least 2 cases`

Bad:

- `The output is good`
- `The skill feels smart`
- `The answer is polished`

Every pass or fail must include evidence.

## Output Format

```text
## Eval Report: <skill-name>
- Goal: <goal>
- Structure: PASS/NEEDS_WORK/FAIL
- Live run: PASS/NEEDS_WORK/FAIL
- Baseline: not run / summary

### Assertion Results
- PASS: <assertion> — <evidence>
- FAIL: <assertion> — <evidence>

### Issues
- <issue>

### Next Fixes
1. <fix>
2. <fix>

### Verdict
PASS | NEEDS_WORK | FAIL — <one-sentence summary>
```

## References

- `references/evaluating-skills.md` — condensed eval workflow and grading guidance
