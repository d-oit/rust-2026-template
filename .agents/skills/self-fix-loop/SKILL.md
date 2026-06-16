---
name: self-fix-loop
description: >
  Self-learning fix loop - commit, push, monitor CI, auto-fix failures using
  swarm agents with skills on demand, loop until all checks pass.
  Use when CI fails and you need to iteratively fix until green.
  Triggers: "fix CI", "loop until green", "auto-fix failures", "self-fix".
category: workflow
license: MIT
metadata:
  author: d-oit
  version: "0.2.10"
  adapted-from: d-o-hub/github-template-ai-agents
---

# Self-Fix Loop Skill

Automated self-learning cycle: **commit → push → monitor → analyze failures → fix → retry** until all GitHub Actions pass.

**Self-Fix Threshold**: If 2+ similar errors occur during the loop, pause and diagnose the root cause before attempting another fix.

## Overview

Continuous improvement loop that:
1. Commits all changes atomically
2. Pushes to feature branch
3. Creates/updates PR
4. Monitors GitHub Actions
5. On failure: diagnoses and fixes
6. Repeats until ALL checks pass

## Arguments

| Argument | Description | Default |
|----------|-------------|---------|
| `--max-retries N` | Maximum fix iterations | 5 |
| `--auto-research` | Use web research on failures | true |
| `--fix-issues` | Attempt automatic fixes | true |
| `--strict-validation` | ALL checks must pass | true |
| `--timeout SECONDS` | Per-iteration timeout | 1800 |
| `--dry-run` | Simulate without pushing | false |

## Loop Phases

```
[Start]
   ↓
Phase 1: COMMIT & PUSH
   - Stage all changes
   - Run quality gate
   - Atomic commit
   - Push to feature branch
   ↓
Phase 2: CREATE/UPDATE PR
   - Create new PR or update existing
   ↓
Phase 3: MONITOR CI
   - Poll GitHub Actions
   - Wait for all checks complete
   ↓
Phase 4: ANALYZE FAILURES
   - Identify failed checks
   - Extract error messages
   - Categorize failure type
   ↓
Phase 5: FIX (if failures)
   - Diagnose root cause
   - Apply fixes
   - Commit fix
   ↓
Phase 6: RETRY LOOP
   - If retries remaining → Phase 1
   - If max retries → FAIL
   - If all pass → SUCCESS
```

## Rust Failure Types

| Failure Type | Action |
|--------------|--------|
| `cargo fmt` | Run `cargo fmt --all` |
| `cargo clippy` | Fix warnings, respect `-D warnings` |
| `cargo test` | Debug test failures, fix or skip flaky |
| `cargo audit` | Update vulnerable dependency |
| `cargo deny` | Fix license or advisory violations |
| `cargo machete` | Remove unused dependencies |
| Linker errors | Check `build-essential`, `pkg-config` |

## Rationalizations

| Rationalization | Reality |
|-----------------|---------|
| "The loop is stuck, I'll just force merge" | Force merging defeats the purpose of CI and introduces regressions. |
| "One more retry should fix it" | Repeated identical failures indicate a root cause, not a flaky test. |
| "I'll disable strict validation to pass faster" | Strict validation catches issues early; disabling it pushes bugs to production. |

## Red Flags

- [ ] Exceeding max retries without diagnosing root cause
- [ ] Disabling strict validation to bypass failing checks
- [ ] Force merging when the self-fix loop reports failure
