# JSON Schemas

Reference schemas for skill-related JSON files.

## evals.json

Located at `skill-name/evals/evals.json`.

```json
{
  "skill_name": "string (required)",
  "evals": [
    {
      "id": "number (required)",
      "prompt": "string (required) — realistic user prompt",
      "expected_output": "string (required) — short success definition",
      "files": ["string — optional input file paths"],
      "assertions": ["string (required) — concrete, checkable assertions"]
    }
  ]
}
```

### Field Rules

| Field | Required | Notes |
|-------|----------|-------|
| `skill_name` | Yes | Must match the skill directory name |
| `evals` | Yes | Array of 1+ test cases |
| `evals[].id` | Yes | Unique integer per case |
| `evals[].prompt` | Yes | Realistic user prompt with context |
| `evals[].expected_output` | Yes | Brief description of expected result |
| `evals[].files` | No | Input files the skill should read |
| `evals[].assertions` | Yes | 1+ concrete, checkable assertions |

### Example

```json
{
  "skill_name": "build-rust",
  "evals": [
    {
      "id": 1,
      "prompt": "Build this Rust project and fix any issues",
      "expected_output": "Runs cargo check, build, clippy, fmt and reports results",
      "files": [],
      "assertions": [
        "Runs cargo check --all-targets --all-features",
        "Runs cargo clippy with -D warnings",
        "Runs cargo fmt --check"
      ]
    }
  ]
}
```

## grading.json

Located at `skill-name/evals/grading.json` (optional, for benchmark runs).

```json
{
  "skill_name": "string (required)",
  "run_id": "string (required) — unique run identifier",
  "timestamp": "string (required) — ISO 8601",
  "results": [
    {
      "eval_id": "number (required)",
      "passed": "boolean (required)",
      "evidence": "string (required) — what was observed",
      "assertions": [
        {
          "assertion": "string (required)",
          "passed": "boolean (required)",
          "evidence": "string (required)"
        }
      ]
    }
  ],
  "summary": {
    "total": "number (required)",
    "passed": "number (required)",
    "failed": "number (required)",
    "pass_rate": "number (required) — 0.0 to 1.0"
  }
}
```

### Field Rules

| Field | Required | Notes |
|-------|----------|-------|
| `skill_name` | Yes | Must match the skill directory name |
| `run_id` | Yes | UUID or timestamp-based identifier |
| `timestamp` | Yes | ISO 8601 format |
| `results` | Yes | One entry per eval case |
| `results[].eval_id` | Yes | Matches evals.json id |
| `results[].passed` | Yes | Overall pass/fail for this case |
| `results[].evidence` | Yes | What was observed during the run |
| `results[].assertions` | Yes | Per-assertion results |
| `summary` | Yes | Aggregate statistics |
| `summary.pass_rate` | Yes | Float between 0.0 and 1.0 |

## Validation Checklist

Before committing evals.json:
- [ ] Valid JSON (no trailing commas)
- [ ] `skill_name` matches directory name
- [ ] All cases have `id`, `prompt`, `expected_output`, `assertions`
- [ ] At least 1 eval case exists
- [ ] Assertions are concrete (not "the output is good")
