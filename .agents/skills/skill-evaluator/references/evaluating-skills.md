# Evaluating Skills

Condensed eval workflow and grading guidance for skill evaluation.

## Evaluation Goals

| Goal | When to Use |
|------|-------------|
| Structure check | First pass on any skill; validates directory layout and required files |
| Eval review | Assess whether evals/evals.json cases are realistic and assertion-rich |
| Live run | Execute a real prompt through the skill and grade the output |
| Baseline comparison | Measure improvement against a no-skill or older-skill snapshot |

## Grading Rules

### Assertions

Every pass or fail must include concrete evidence.

**Good assertions:**
- `The answer cites the exact minimum cover dimensions`
- `The output includes all 7 scoring dimensions`
- `evals.json contains at least 2 cases`

**Bad assertions:**
- `The output is good`
- `The skill feels smart`
- `The answer is polished`

### Verdict Criteria

| Verdict | Criteria |
|---------|----------|
| `PASS` | Structure is sound and live output meets all assertions |
| `NEEDS_WORK` | Usable, but structure gaps or output gaps remain |
| `FAIL` | Skill is broken, misleading, or missing core pieces |

## Workflow Steps

### 1. Structure Check

Confirm the skill directory layout:

```
skill-name/
  SKILL.md           # Required
  evals/evals.json   # Recommended
  references/        # Recommended
  scripts/           # Optional
```

Flag:
- Missing `SKILL.md`
- Nested duplicate directories
- `evals/` exists but `evals.json` is missing or invalid JSON
- Eval cases missing `id`, `prompt`, or `expected_output`

### 2. Eval Review

Read `evals/evals.json` and assess each case:

**Strong evals include:**
- A realistic user prompt (with file paths, context, casual language)
- A short success definition
- Concrete, checkable assertions
- Optional input files

**Weak evals include:**
- Vague prompts
- Purely subjective assertions
- No evidence path for pass/fail

### 3. Live Run

For each representative prompt:
1. Load the target skill
2. Read only the files the skill points to
3. Produce the answer or output
4. Grade against assertions with evidence

### 4. Baseline Comparison

When measuring improvement, rerun the same prompt without the skill or against an older snapshot. Compare:
- Pass rate
- Missing details
- Format compliance
- Token cost (if available)

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
