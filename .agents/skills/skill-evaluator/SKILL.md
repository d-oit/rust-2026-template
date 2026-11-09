---
name: skill-evaluator
description: "Reusable skill for evaluating other skills with structure checks, eval coverage review, and real usage spot checks. Use when you need to check a skill, add evals, benchmark a skill, validate outputs against assertions, or compare current skill behavior against a baseline."

license: MIT
metadata:
  category: meta
  author: d-oit
  version: "1.1"
  source: d-o-hub/github-template-ai-agents
  spec: "agentskills.io"
---

# Skill Evaluator

Evaluate local skills with a repeatable loop: inspect structure, read eval definitions,
run one or more realistic prompts, then score the output with explicit assertions and evidence.

## Required Inputs

At minimum, identify:

```text
SKILL_PATH: absolute or workspace-relative path to the skill directory
GOAL: structure check / eval review / live run / baseline comparison
```

## Evaluation Workflow

### 1. Structure Check

Confirm the skill directory is sane before judging outputs.

Run the repository validator first — it encodes hard checks you would
otherwise have to re-derive:

```bash
bash scripts/validate-skills.sh   # exit 2 on failure
./.agents/skills-evaluation/scripts/structure_check.sh <iteration-dir>  # scored report
```

`validate-skills.sh` hard-fails on: missing `SKILL.md`, frontmatter `name`
missing or not matching the directory name, non-spec frontmatter keys
(only `name`, `description`, `license`, `compatibility`, `metadata`,
`allowed-tools` are legal), missing description, and body over 250 lines.
It warns (does not fail) on missing `metadata.category`, `## Rationalizations`,
`## Red Flags`, or `evals/evals.json`.

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

Assess skill performance from these sources (in order):

- `.agents/events/YYYY/MM/DD/*.json` — per-task events written by the
  `metrics-reporter` skill (the canonical input).
- `bash scripts/aggregate-metrics.sh` — rolls events up into
  `.agents/aggregated/metrics.jsonl`; run it first if the rollup looks stale.
- `.agents/metrics.jsonl` — legacy append-only log; consult it when the
  rollup predates an event.

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

Run at least one representative prompt from the eval set or create a focused ad
hoc prompt. `bash scripts/run-evals.sh --skill <name>` only validates eval JSON
structure — it does not execute prompts, so a PASS from it is not a live run.
CI (`.github/workflows/eval.yml`) runs the same structure validation weekly;
grading still happens here.

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

- `../skill-creator/references/evaluating-skills.md` — the canonical eval workflow and
  grading guidance. This is a sibling copy removed to stop the two drifting.

## Rationalizations

| Rationalization | Reality |
|-----------------|---------|
| "The skill works fine, no need to evaluate." | Unvalidated skills drift. Regular evaluation catches regressions early. |
| "One eval case is enough." | One case can't cover edge scenarios. Aim for 3+ diverse evals per skill. |
| "I'll use subjective grading." | Subjective grading is unreliable. Use concrete, checkable assertions. |

## Red Flags

- [ ] Evaluating a skill without a clear GOAL (structure check / eval review / live run)
- [ ] Using vague assertions like "the output is good"
- [ ] Not comparing against a baseline when measuring improvement
