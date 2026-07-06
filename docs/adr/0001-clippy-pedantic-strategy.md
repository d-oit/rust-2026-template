# ADR 0001: Clippy Pedantic Lint Strategy

**Date:** 2026-07-06  
**Status:** Accepted

## Context

This workspace uses AI agents (Claude, Gemini, Qwen) for code generation.
Agents under Clippy pressure tend to silence lints via `#[allow(clippy::...)]`
rather than fixing the underlying code. This degrades code quality silently
over time and defeats the purpose of the linter.

## Decision

1. `allow_attributes = "deny"` is set in `[workspace.lints.clippy]` in `Cargo.toml`.
   This makes any `#[allow(clippy::...)]` annotation a **compile error**, forcing
   agents (and contributors) to fix code instead of suppressing diagnostics.

2. `unwrap_used` and `expect_used` are set to `"deny"` (not `"warn"`). Production
   code must use proper `?`-propagation or explicit match/map_err patterns.

3. `pedantic` remains at `"allow"` globally, with individual high-value pedantic
   lints promoted to `"warn"` explicitly. This avoids noisy churn while capturing
   the most impactful correctness signals.

4. Test modules may use module-level `#![allow(clippy::unwrap_used, clippy::expect_used)]`
   at the top of `#[cfg(test)] mod tests { ... }` blocks. Per-call-site
   suppression is blocked by rule 1.

## Consequences

- AI agents cannot silently suppress Clippy violations — they must fix code.
- CI will fail on any `#[allow(clippy::...)]` attribute introduced in a PR.
- New lints promoted from pedantic require an explicit `Cargo.toml` entry
  (which is protected by the `guard-lint-config.sh` PreToolUse hook).
- Slightly higher initial friction when onboarding new crates — acceptable trade-off.

## Supersedes

N/A — initial decision.
