---
name: issue-triage
description: >
  Read all open GitHub issues, categorize by type and effort, plan batch
  implementation order, and coordinate implementation across multiple issues
  in a single PR. Use when asked to "implement all issues", "fix all open
  issues", "batch implement", or "address all issues".
  Triggers: "all issues", "implement all", "batch issues", "open issues",
  "fix all", "address all issues".
category: workflow
license: MIT
metadata:
  author: d-oit
  version: "0.1.0"
---

# Issue Triage & Batch Implementation Skill

Reads all open GitHub issues, categorizes them, plans implementation order, and coordinates batch implementation in a single PR.

## Overview

Workflow that orchestrates issue discovery → categorization → implementation planning → batch execution → CI verification → PR merge.

**Single PR policy**: All issues implemented in one PR unless user specifies otherwise.

## Arguments

| Argument | Description | Default |
|----------|-------------|---------|
| `--repo OWNER/REPO` | GitHub repository | auto-detect from git remote |
| `--state open\|closed\|all` | Issue filter | open |
| `--limit N` | Max issues to process | 50 |
| `--batch-size N` | Issues per PR | all |
| `--dry-run` | Plan only, no implementation | false |
| `--skip-merge` | Create PR but don't merge | false |

## Workflow Phases

```
[Start]
   ↓
Phase 1: DISCOVER
   - gh issue list --state open
   - Read issue bodies, labels, acceptance criteria
   ↓
Phase 2: CATEGORIZE
   - Group by type: feature, bugfix, docs, ci, chore
   - Group by effort: trivial, small, medium, complex
   - Identify dependencies between issues
   - Flag already-implemented issues
   ↓
Phase 3: PLAN
   - Sort by dependency order (dependencies first)
   - Group compatible issues for single PR
   - Estimate risk level per group
   - Create implementation checklist
   ↓
Phase 4: IMPLEMENT
   - Execute implementation groups in order
   - For each group: code → tests → docs
   - Track progress per issue
   ↓
Phase 5: VERIFY
   - Run quality gates
   - Push and create PR
   - Monitor CI until green
   - Fix any failures
   ↓
Phase 6: REPORT
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
| > 500 LOC change | Complex | Individual PR consideration |

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
| REPORT | PR created and green | Done |

## Integration with Other Skills

- **`atomic-commit`**: Used in Phase 5 for PR creation
- **`self-fix-loop`**: Used in Phase 5 if CI fails
- **`task-decomposition`**: Used in Phase 3 for complex issues
- **`build-rust`**: Used in Phase 4 for compilation checks
- **`lint-rust`**: Used in Phase 4 for quality checks
- **`test-rust`**: Used in Phase 4 for test verification

## Output Format

```markdown
## Issue Triage Report

### Summary
- Total open: N
- To implement: M
- Already done: K
- Skipped (too complex): L

### Implementation Plan
| # | Issue | Type | Effort | Status |
|---|-------|------|--------|--------|
| 1 | #123 | feature | small | ✅ done |
| 2 | #124 | docs | trivial | ✅ done |

### Skipped Issues
- #125 (complex feature, needs design discussion)

### CI Status
- All checks: ✅ pass
- PR: https://github.com/.../pull/XXX
```

## Example Usage

```
User: "read all open github issues and implement all missing"
Agent: Uses issue-triage skill → discover → categorize → plan → implement → verify → report

User: "implement issues #100-#110 in one PR"
Agent: Uses issue-triage with --limit 11 → filters to specific issues → batch implement

User: "triage open issues and tell me what's already done"
Agent: Uses issue-triage with --dry-run → discover → categorize → report (no implementation)
```

## Rationalizations

| Misconception | Reality |
|---------------|---------|
| "Just implement issues one by one" | Batching in a single PR reduces CI overhead and ensures consistent implementation |
| "Skip already-implemented issues silently" | Always report which issues were already done so the user knows |
| "Complex issues should wait" | Flag them but include in the plan with appropriate warnings |

## Red Flags

- [ ] Implementing issues without reading their full acceptance criteria
- [ ] Creating separate PRs for each issue when user asked for batch
- [ ] Skipping dependency analysis between issues
- [ ] Not reporting which issues were already implemented
- [ ] Ignoring CI failures after implementation
