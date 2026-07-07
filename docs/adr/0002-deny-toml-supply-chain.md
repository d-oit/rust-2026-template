# ADR 0002: Supply Chain Security via cargo-deny

**Date:** 2026-07-06  
**Status:** Accepted

## Context

Rust's crate ecosystem is large and dependency chains are deep. Supply chain
attacks (malicious crates, license violations, known CVEs) are a real risk
for any production template that will be forked and built upon.

## Decision

1. `cargo deny` is the primary supply chain gate, configured in `deny.toml`.
   It enforces license allowlists, blocks known-vulnerable crate versions (via
   RustSec advisory DB), and enforces crate duplication limits.

2. `deny.toml` is a **protected file** — it cannot be modified by AI agents
   (enforced by `.claude/hooks/guard-lint-config.sh` PreToolUse hook).
   Changes require human review and a new ADR entry if the policy changes.

3. `.gitleaks.toml` provides secret scanning as a complementary control.
   It runs in CI and as a pre-commit hook.

4. `cargo deny check` is part of the CI pipeline and is a required status check
   before merge.

## Consequences

- Any new dependency with a disallowed license will fail CI immediately.
- Known-vulnerable transitive dependencies will block merges until updated.
- Agents cannot weaken supply chain policy to resolve dependency conflicts —
  they must find compatible crate versions instead.
- Periodic manual review of `deny.toml` is required as the advisory DB evolves.

## Supersedes

N/A — initial decision.
