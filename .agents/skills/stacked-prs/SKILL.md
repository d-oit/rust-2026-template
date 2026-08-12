---
name: stacked-prs
description: >
  Optional workflow for large features using stacked PRs via gh-stack.
  Use ONLY when a feature has 3+ distinct concerns with clear dependencies.
  NOT for bug fixes, hotfixes, or small changes. Requires GitHub.
  Triggers: "stacked PRs", "gh stack", "split into layers", "stack this".
category: workflow
license: MIT
metadata:
  author: d-oit
  version: "0.1.0"
  optional: true
  upstream: https://github.com/github/gh-stack
---

# Stacked PRs (Optional)

## When to Use

Stacked PRs are OPTIONAL. Default to single-PR atomic commits (see `atomic-commit` skill).
Use stacks ONLY when ALL of these hold:

- Feature has 3+ distinct concerns with clear dependencies
- Different layers need different reviewers
- Bottom layers can merge independently while top layers are WIP
- Repository is on GitHub (not GitLab)

Do NOT use for: bug fixes, hotfixes, small/medium changes, or issue-triage batch work.

## Setup

```bash
gh extension install github/gh-stack
gh skill install github/gh-stack    # full agent skill with references
git config rerere.enabled true      # remember conflict resolutions
```

The full agent skill (SKILL.md + references/) is installed by `gh skill install`.
This file provides template-specific integration guidance only.

## Decision Flow

1. Is this a bug fix, hotfix, or small change? → Use `atomic-commit` (single PR)
2. Is this on GitLab? → Use `atomic-commit` (gh-stack is GitHub-only)
3. Does the feature have 3+ distinct layers? → Consider stacking
4. Are different reviewers needed per layer? → Stack makes sense
5. Otherwise → Default to single PR

## Template Integration

- **Quality gates** run per-PR naturally — each layer gets its own CI via `./scripts/quality-gates.sh`
- **Conventional commits** per layer: `feat(scope): description`
- **Branch naming**: `<topic>/<concern>` (e.g., `billing/schema`, `billing/api`)
  - Compatible with `feat/` prefix: `feat/billing/schema`
  - Compatible with `fix/` prefix: `fix/auth/validation`
- **Metrics**: Report one event per layer completion to `.agents/events/YYYY/MM/DD/`
- **Hotfixes**: Never stack hotfixes — they must be atomic for DORA Change Failure Rate tracking

## Non-Interactive Agent Usage

Always use flags to avoid TUI blocking:

| Command | Use | Never run bare |
|---------|-----|----------------|
| View | `gh stack view --json` | `gh stack view` (opens TUI) |
| Submit | `gh stack submit --auto` | `gh stack submit` (prompts per PR) |
| Init | `gh stack init <branches...>` | `gh stack init` (prompts for names) |
| Merge | `gh stack merge <target> --yes` | `gh stack merge` (interactive picker) |
| Add | `gh stack add <branch>` | `gh stack add` (prompts for name) |
| Checkout | `gh stack checkout <target>` | `gh stack checkout` (opens menu) |
| Modify | Not available non-interactively | `gh stack modify` (TUI-only) |

## Typical Agent Workflow

```bash
# 1. Create stack with named branches
gh stack init feat/billing/schema feat/billing/api feat/billing/ui

# 2. Implement bottom layer
git add ... && git commit -m "feat(billing): add schema and models"
./scripts/quality-gates.sh

# 3. Move up and implement next layer
gh stack up
git add ... && git commit -m "feat(billing): add API routes"
./scripts/quality-gates.sh

# 4. Push and create draft PRs
gh stack submit --auto

# 5. After review changes on a layer
gh stack bottom
git add ... && git commit -m "fix(billing): address schema review"
gh stack rebase --upstack
gh stack push
```
