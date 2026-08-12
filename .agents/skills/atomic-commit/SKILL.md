---
name: atomic-commit
description: >
  Atomic git workflow - validates, commits, pushes, creates PR/MR, and verifies CI
  with zero-warnings policy. Orchestrates complete code submission as state machine
  with rollback on failure. Supports GitHub (gh) and GitLab (glab).
  Triggers: "commit changes", "push and create PR", "submit code", "atomic commit".
category: workflow
license: MIT
metadata:
  author: d-oit
  version: "0.3.0"
  adapted-from: d-o-hub/github-template-ai-agents
---

# Atomic Commit Skill

## When to Use

- User asks for this skill's functionality

Atomic workflow: validate → commit → push → PR/MR → verify. All changes committed as single unit with **zero warnings** policy.

**For large features (3+ distinct concerns)**, consider the `stacked-prs` skill instead.

## Overview

Orchestrates complete code submission as state machine with 7 phases:
1. PRE_COMMIT - Validation (quality gate, secrets scan)
2. COMMIT - Atomic commit creation (conventional format)
3. PRE_PUSH - Remote sync check
4. PUSH - Upload to origin
5. PR_CREATE - Open pull request (GitHub) or merge request (GitLab)
6. VERIFY - Wait for CI checks
7. REPORT - Success summary

**Zero warnings policy**: Any warning fails the workflow and triggers rollback.

## Platform Detection

Auto-detect from git remote:

```bash
REMOTE=$(git remote get-url origin)
if echo "$REMOTE" | grep -qi "github"; then
  PLATFORM="github"; CLI="gh"
elif echo "$REMOTE" | grep -qi "gitlab"; then
  PLATFORM="gitlab"; CLI="glab"
fi
```

## CLI Command Mapping

| Action | GitHub (`gh`) | GitLab (`glab`) |
|--------|---------------|-----------------|
| Create PR/MR | `gh pr create --title "..."` | `glab mr create --title "..."` |
| Check CI | `gh pr checks` | `glab ci status` |
| Merge | `gh pr merge 123 --squash` | `glab mr merge 123 --squash` |

## Arguments

| Argument | Description | Default |
|----------|-------------|---------|
| `--message, -m` | Commit message (auto-detect if omitted) | auto |
| `--platform github\|gitlab` | Force platform | auto-detect |
| `--dry-run` | Validate only, no commits/pushes | false |
| `--skip-ci` | Skip CI verification | false |
| `--timeout` | CI wait timeout in seconds | 1800 |
| `--base-branch` | Target branch for PR/MR | main |

## State Machine

```
[Start] → PRE_COMMIT → COMMIT → PRE_PUSH → PUSH → PR_CREATE → VERIFY → REPORT → [Success]
              ↓           ↓         ↓         ↓          ↓         ↓
            [Fail]      Rollback  Rollback  Rollback   Rollback  Rollback
```

## Quality Gates

| Phase | Check | Failure Action |
|-------|-------|----------------|
| PRE_COMMIT | `./scripts/quality-gates.sh` zero warnings | Abort |
| PRE_COMMIT | No secrets in diff | Abort |
| COMMIT | Valid conventional format | Rollback |
| PRE_PUSH | Remote accessible | Rollback |
| PUSH | SHA verification | Rollback |
| PR_CREATE | gh/glab CLI authenticated | Rollback |
| VERIFY | All CI checks green | Rollback |

## Rust-Specific Checks

Before committing, verify:
- `cargo fmt --check` passes
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `cargo nextest run --workspace` passes
- No `unsafe` blocks without `// SAFETY:` comments

## Commit Format

```
type(scope): Brief description (50 chars max)

- Why (not what) - user perspective
- Reference issues: Fixes #123
```

**Types:** feat, fix, docs, style, refactor, perf, test, ci, chore

## Rationalizations

| Rationalization | Reality |
|-----------------|---------|
| "I'll skip the quality gate, it's just a small change" | Small changes cause the majority of production incidents. |
| "Rollback is too complicated, I'll just fix forward" | Fix-forward creates tangled history and makes it harder to isolate what broke. |
| "I don't need to verify CI, the changes are cosmetic" | Cosmetic changes can still break builds, linters, or type checks. |

## Red Flags

- [ ] Bypassing the quality gate for "trivial" changes
- [ ] Committing without running validation checks
- [ ] Disabling rollback on failure
