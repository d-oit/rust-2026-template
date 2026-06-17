---
name: issue-triage
description: >
  Read all open issues (GitHub/GitLab), categorize by type and effort, plan
  batch implementation order, and coordinate implementation across multiple
  issues in a single PR/MR. Auto-detects platform from git remote.
  Use when asked to "implement all issues", "fix all open issues",
  "batch implement", or "address all issues".
  Triggers: "all issues", "implement all", "batch issues", "open issues",
  "fix all", "address all issues".
category: workflow
license: MIT
metadata:
  author: d-oit
  version: "0.2.0"
---

# Issue Triage & Batch Implementation Skill

Reads all open issues, categorizes them, plans implementation order, and coordinates batch implementation in a single PR/MR. Supports GitHub and GitLab.

## Overview

Workflow that orchestrates issue discovery → categorization → implementation planning → batch execution → CI verification → PR/MR merge.

**Single PR/MR policy**: All issues implemented in one PR/MR unless user specifies otherwise.

## Platform Detection

Auto-detect from git remote:

```bash
REMOTE=$(git remote get-url origin)
if echo "$REMOTE" | grep -qi "github"; then
  PLATFORM="github"; CLI="gh"
elif echo "$REMOTE" | grep -qi "gitlab"; then
  PLATFORM="gitlab"; CLI="glab"
else
  echo "Unknown platform - set --platform github|gitlab"
fi
```

## CLI Command Mapping

| Action | GitHub (`gh`) | GitLab (`glab`) |
|--------|---------------|-----------------|
| List issues | `gh issue list --state open` | `glab issue list --state opened` |
| Read issue | `gh issue view 123` | `glab issue view 123` |
| Create PR/MR | `gh pr create --title "..."` | `glab mr create --title "..."` |
| Check CI | `gh pr checks` | `glab ci status` |
| Merge | `gh pr merge 123 --squash` | `glab mr merge 123 --squash` |
| Close issue | `gh issue close 123` | `glab issue close 123` |

## Arguments

| Argument | Description | Default |
|----------|-------------|---------|
| `--repo OWNER/REPO` | Repository | auto-detect from git remote |
| `--platform github\|gitlab` | Force platform | auto-detect |
| `--state open\|closed\|all` | Issue filter | open (GitHub) / opened (GitLab) |
| `--limit N` | Max issues to process | 50 |
| `--batch-size N` | Issues per PR/MR | all |
| `--dry-run` | Plan only, no implementation | false |
| `--skip-merge` | Create PR/MR but don't merge | false |

## Prerequisites

- `gh` CLI authenticated (`gh auth status`) — for GitHub
- `glab` CLI authenticated (`glab auth status`) — for GitLab
- Repository hosted on GitHub or GitLab
- CI enabled (GitHub Actions / GitLab CI) for verification

## Workflow Phases

```
[Start]
   ↓
Phase 1: DETECT PLATFORM
   - Check git remote URL
   - Select gh or glab CLI
   ↓
Phase 2: DISCOVER
   - gh issue list / glab issue list
   - Read issue bodies, labels, acceptance criteria
   ↓
Phase 3: CATEGORIZE
   - Group by type: feature, bugfix, docs, ci, chore
   - Group by effort: trivial, small, medium, complex
   - Identify dependencies between issues
   - Flag already-implemented issues
   ↓
Phase 4: PLAN
   - Sort by dependency order (dependencies first)
   - Group compatible issues for single PR/MR
   - Estimate risk level per group
   - Create implementation checklist
   ↓
Phase 5: IMPLEMENT
   - Execute implementation groups in order
   - For each group: code → tests → docs
   - Track progress per issue
   ↓
Phase 6: VERIFY
   - Run quality gates
   - Push and create PR/MR
   - Monitor CI until green
   - Fix any failures
   ↓
Phase 7: REPORT
   - Summarize implemented issues
   - List any skipped/blocked issues
   - Report CI status
```

## Categorization Rules

### By Type

| Label Pattern | Category | Implementation Approach |
|---------------|----------|------------------------|
| `feature`, `enhancement` | Feature | New code + tests + docs |
| `bug`, `bugfix` | Bugfix | Fix + regression test |
| `documentation`, `docs` | Docs | README/md updates only |
| `ci`, `chore`, `dependencies` | Infrastructure | Config/script changes |
| `security` | Security | Audit + fix + verify |

### By Effort

| Signal | Effort | Batch Strategy |
|--------|--------|----------------|
| < 50 LOC change | Trivial | Batch with any other issue |
| 50-200 LOC change | Small | Batch with 2-3 similar issues |
| 200-500 LOC change | Medium | Individual or pair |
| > 500 LOC change | Complex | Individual PR/MR consideration |

### Already-Implemented Detection
Skip issues where:
- Source code already exists with the described functionality
- Tests already cover the acceptance criteria
- README already documents the feature

## Dependency Detection

Look for:
- Issue mentions another issue number (e.g., "depends on #123")
- Acceptance criteria references files created by another issue
- Feature requires infrastructure from another issue

## Quality Gates

| Phase | Check | Failure Action |
|-------|-------|----------------|
| PLAN | All issues categorized | Re-scan with labels |
| IMPLEMENT | Each change compiles | Fix before proceeding |
| VERIFY | `./scripts/quality-gates.sh` | Fix and retry |
| VERIFY | All CI checks pass | `self-fix-loop` skill |
| REPORT | PR/MR created and green | Done |

## Integration with Other Skills

- **`atomic-commit`**: Used in Phase 6 for PR/MR creation
- **`self-fix-loop`**: Used in Phase 6 if CI fails
- **`task-decomposition`**: Used in Phase 4 for complex issues
- **`build-rust`**: Used in Phase 5 for compilation checks
- **`lint-rust`**: Used in Phase 5 for quality checks
- **`test-rust`**: Used in Phase 5 for test verification

## Output Format

```markdown
## Issue Triage Report

**Platform:** GitHub (gh) / GitLab (glab)

### Summary
- Total open: N
- To implement: M
- Already done: K
- Skipped (too complex): L

### Implementation Plan
| # | Issue | Type | Effort | Status |
|---|-------|------|--------|--------|
| 1 | #123 | feature | small | done |
| 2 | #124 | docs | trivial | done |

### Skipped Issues
- #125 (complex feature, needs design discussion)

### CI Status
- All checks: pass
- PR/MR: https://github.com/.../pull/XXX  or  https://gitlab.com/.../-/merge_requests/XXX
```

## Example Usage

```
User: "read all open github issues and implement all missing"
Agent: Detects GitHub → uses gh → discover → categorize → plan → implement → verify → report

User: "implement all issues on gitlab"
Agent: Detects GitLab → uses glab → discover → categorize → plan → implement → verify → report

User: "implement issues #100-#110 in one MR"
Agent: Uses issue-triage with --limit 11 → filters to specific issues → batch implement

User: "triage open issues and tell me what's already done"
Agent: Uses issue-triage with --dry-run → discover → categorize → report (no implementation)
```

## Rationalizations

| Misconception | Reality |
|---------------|---------|
| "Just implement issues one by one" | Batching in a single PR/MR reduces CI overhead and ensures consistent implementation |
| "Skip already-implemented issues silently" | Always report which issues were already done so the user knows |
| "Complex issues should wait" | Flag them but include in the plan with appropriate warnings |
| "Platform doesn't matter" | Commands differ between GitHub/GitLab; auto-detection prevents errors |

## Red Flags

- [ ] Implementing issues without reading their full acceptance criteria
- [ ] Creating separate PRs/MRs for each issue when user asked for batch
- [ ] Skipping dependency analysis between issues
- [ ] Not reporting which issues were already implemented
- [ ] Ignoring CI failures after implementation
- [ ] Using wrong CLI for the platform (gh on GitLab or vice versa)
