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
git config rerere.enabled true      # remember conflict resolutions
```

See https://github.com/github/gh-stack for the full agent skill (SKILL.md + references/).
This file provides template-specific integration guidance only.

## Decision Flow

Use stacked PRs when: GitHub repo, 3+ distinct layers, different reviewers per layer.
Otherwise: use `atomic-commit` (single PR).

## Template Integration

- **Quality gates** run per-PR naturally — each layer gets its own CI via `./scripts/quality-gates.sh`
- **Conventional commits** per layer: `feat(scope): description`
- **Branch naming**: `<topic>/<concern>` (e.g., `billing/schema`, `billing/api`)
- **Metrics**: Report one event per layer completion to `.agents/events/YYYY/MM/DD/`
- **Hotfixes**: Never stack hotfixes — they must be atomic for DORA Change Failure Rate tracking

## Non-Interactive Agent Usage

Always use flags to avoid TUI blocking:

| Command | Use | Never run bare |
|---------|-----|----------------|
| View | `gh stack view --json` | `gh stack view` (opens TUI) |
| Submit | `gh stack submit --auto` | `gh stack submit` (prompts per PR) |
| Submit (ready) | `gh stack submit --auto --open` | creates non-draft PRs |
| Init | `gh stack init <branches...>` | `gh stack init` (prompts for names) |
| Merge | `gh stack merge <pr-number> --yes` | `gh stack merge` (interactive picker) |
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

## Merge Strategy

- Always merge bottom-up (closest to trunk first)
- If a bottom PR is rejected, close upstack PRs and restack
- Use `gh stack merge <pr-number> --yes` to merge that PR and every unmerged PR below it
- After all layers merge, `gh stack sync --prune` cleans up local branches

## Failure Recovery

- **Partial submit**: If `gh stack submit` fails mid-way (e.g., network error after 2 of 3 PRs created), re-run `gh stack submit` — it skips branches that already have PRs
- **Rebase conflict**: Run `gh stack rebase`, resolve conflicts, `git add`, `gh stack rebase --continue`. Use `--abort` to restore all branches
- **Diverged stacks**: Run `gh stack sync` to fetch and reconcile with GitHub. If that fails, `gh stack unstack` then `gh stack submit --auto` to recreate.

## Rationalizations

| Rationalization | Reality |
|-----------------|---------|
| "I'll put everything in one PR, it's simpler" | Large PRs get superficial reviews. Stacked PRs get focused, thorough reviews per layer. |
| "Stacking is too much overhead for this feature" | If you're already splitting concerns across files, stacking formalizes what you're doing anyway. |
| "I'll split the PR after I finish coding" | Splitting after the fact is expensive and error-prone. Plan layers before writing code. |

## Red Flags

- [ ] Stacking a bug fix or hotfix (use atomic-commit instead)
- [ ] Using `git push --force` on stacked branches (use `gh stack push`)
- [ ] Stacking on GitLab (gh-stack is GitHub-only)
- [ ] Amending commits on a stacked branch (breaks upstack branches)
