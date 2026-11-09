---
name: lint-rust
description: >
  Run comprehensive linting and static analysis on Rust code including clippy,
  format check, security audit, supply chain, and unused dependencies.
  Use before committing, during CI, or when reviewing code quality.
  Triggers: "lint rust", "clippy", "static analysis", "code quality".

license: MIT
metadata:
  category: rust
  author: d-oit
  version: "1.0"
  tags: rust lint clippy audit deny machete
---

# Skill: lint-rust

## Purpose

Run comprehensive linting and static analysis on Rust code.

## Trigger Conditions

- Before committing code (pre-commit runs `harness-check.sh fmt`;
  clippy and deny run on pre-push — see `.pre-commit-config.yaml`)
- During CI/CD pipeline
- When reviewing code quality
- After adding new dependencies

## Prerequisites

- `cargo clippy` (bundled with Rust)
- `cargo audit` (`cargo install cargo-audit`)
- `cargo deny` (`cargo install cargo-deny`)
- `cargo machete` (`cargo install cargo-machete`)

## Steps

### 0. Canonical wrapper (preferred over ad-hoc runs)

```bash
./scripts/quality-gates.sh
```

Thin wrapper over `cargo run -p xtask --bin xtask -- quality run` — the
canonical quality command. `config/xtask.json` selects the check set per tier
(`pull-request`, `protected-branch` = default, `scheduled`, `release`); all
four tiers include `Audit`, `Deny`, and `Machete`. Use the individual steps
below when a single check fails or you need one sensor fast.

### 1. Clippy strict mode

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

### 2. Format check

```bash
cargo fmt --all -- --check
```

### 3. Security audit

```bash
cargo audit
```

Advisory/ignore policy lives in `.cargo/audit.toml`. CI runs this on every PR
(xtask `Audit` check) and weekly via `security-scan.yml` together with `cargo deny`.

### 4. Supply chain check

```bash
cargo deny check
```

License/bans/advisories/sources policy in `deny.toml`. CI runs it on every PR
(xtask `Deny` check); `security-scan.yml` runs the full scan weekly.

### 5. Unused dependencies

```bash
cargo machete
```

Note: `Machete` belongs to the `protected-branch`, `scheduled`, and `release`
tiers — it is intentionally **not** in the `pull-request` tier, so it is not
part of the fast PR path.

## Success Criteria

- All clippy lints pass with `-D warnings`
- Format is consistent
- No known security vulnerabilities
- All licenses are allowed
- No unused dependencies

## Related Skills

- `build-rust` - Building the project
- `test-rust` - Running tests
- `release-rust` - Release preparation

## Rationalizations

| Rationalization | Reality |
|-----------------|---------|
| "Clippy passes, so the code is clean." | Clippy alone doesn't check licenses, security, or unused deps. Run the full suite. |
| "cargo audit shows no advisories, we're safe." | Audit only checks known advisories. Also run cargo-deny for license compliance. |
| "I'll skip machete — unused deps aren't a big deal." | Unused deps increase compile time and attack surface. Remove them. |

## Red Flags

- [ ] Running only `cargo clippy` without `cargo fmt --check`
- [ ] Skipping `cargo audit` or `cargo deny` in CI
- [ ] Ignoring `-D warnings` flag (allows warnings to accumulate)

## References

- [Clippy lints](https://rust-lang.github.io/rust-clippy/master/)
- [cargo-audit](https://crates.io/crates/cargo-audit)
- [cargo-deny](https://embarkstudios.github.io/cargo-deny/)
