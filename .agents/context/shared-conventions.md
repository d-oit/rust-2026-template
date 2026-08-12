# Shared Agent Conventions (Cross-Repo)

These conventions apply to all repositories derived from `rust-2026-template`.

## Commit Message Format

- Always use Conventional Commits: `feat|fix|chore|docs|test|refactor(scope): description`
- Include `[skip ci]` for docs-only or metrics-only commits
- Body lines should not contain backticks (commitlint rejects them)

## Branch Naming

- Features: `feat/short-description`
- Fixes: `fix/issue-number-description`
- Hotfixes: `hotfix/critical-description` (triggers Change Failure Rate tracking)
- Stacked branches: `<topic>/<concern>` (e.g., `billing/schema`, `billing/api`, `billing/ui`)
  Compatible with `feat/` prefix: `feat/billing/schema`

## PR Requirements

- All PRs must pass `./scripts/quality-gates.sh` locally before opening
- PRs fixing regressions must include a test that reproduces the failure
- Commit messages must pass commitlint (conventional format, no backticks in body)

## Agent Behavior

- Always read `.agents/ci/ci-summary.md` before starting work
- Always append to `.agents/events/YYYY/MM/DD/` after task completion (see metrics-reporter skill)
- Never commit secrets or API keys; run `gitleaks` if unsure
- Use `./scripts/bootstrap.sh` for first-time setup in new clones

## Code Quality

- Max 500 LOC per source file
- No `unwrap()` in library code (test code is acceptable)
- Use `thiserror` for library errors, `anyhow` for application errors
- Enforce `#![forbid(unsafe_code)]` at workspace and crate roots
