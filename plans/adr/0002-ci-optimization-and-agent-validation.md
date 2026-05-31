# ADR 0002: CI Optimization and Agent Entrypoint Validation

## Status
Proposed

## Context
The repository follows a single-source-of-truth model for agent guidance via `AGENTS.md`.
Assistant-specific entrypoints (`CLAUDE.md`, `GEMINI.md`, `QWEN.md`, etc.) are "thin references"
that point to `AGENTS.md`.

Currently:
1. There is no automated validation that these thin references stay "thin" and correct.
2. CI runs the full suite of jobs (including heavy jobs like benchmarks and MSRV checks) on
every PR, regardless of whether the changes affect those areas (e.g., docs-only changes).
3. We lack a recorded baseline for CI performance to measure improvements.

## Decision
1. **Agent Entrypoint Validation**: Implement a validator script and CI job that ensures
assistant-specific entrypoints start with the `@AGENTS.md` directive while allowing specific
tips below.
2. **Path-Based CI Gating**: Tier CI jobs into "Fast Path" (lint, fmt, test, validator) and
"Heavy Path" (bench, msrv, fuzz, mutants). Skip Heavy Path jobs for changes that only affect
documentation or agent guidance.
3. **Thin References over Symlinks**: Explicitly prefer regular files containing `@AGENTS.md`
over filesystem symlinks for maximum portability across all platforms and toolchains.

## Baseline (Pre-Optimization)
- **Total PR Workflow Duration**: ~5-8 minutes (estimated based on full workspace build and test).
- **Heavy Job Frequency**: 100% of PRs trigger all jobs.
- **Agent Entrypoint Integrity**: Manual verification only.
- **Cold Build Time**: ~3-4 minutes (estimated).

## Consequences
- **Positive**: Reduced CI cost and faster feedback loop for documentation and agent guidance
changes. Improved reliability of agent entrypoints.
- **Negative**: Slightly more complex CI configuration. Risk of over-aggressive filtering
if not monitored.

## Documentation
Maintainer guidance will be added to `agents-docs/agent-doc-flow.md`.

## After Rollout Expectations
- **PR Feedback Loop**:
  - Docs-only PRs: < 2 minutes (Lint + Validator only).
  - Code PRs: ~5-8 minutes (Full suite).
- **Cost Savings**: ~30-50% reduction in total CI minutes for typical maintenance cycles
that involve documentation or agent guidance updates.
- **Reliability**: 100% automated enforcement of the @AGENTS.md reference model for agent
entrypoints.
