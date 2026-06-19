---
name: lint-rust
description: >
  Run comprehensive linting and static analysis on Rust code including clippy,
  format check, security audit, supply chain, and unused dependencies.
  Use before committing, during CI, or when reviewing code quality.
  Triggers: "lint rust", "clippy", "static analysis", "code quality".
category: rust
license: MIT
metadata:
  author: d-oit
  version: "1.0"
  tags: rust lint clippy audit deny machete
---

# Skill: lint-rust

## When to Use

- User asks for this skill's functionality


## Purpose

Run comprehensive linting and static analysis on Rust code.

## Trigger Conditions

- Before committing code
- During CI/CD pipeline
- When reviewing code quality
- After adding new dependencies

## Prerequisites

- `cargo clippy` (bundled with Rust)
- `cargo audit` (`cargo install cargo-audit`)
- `cargo deny` (`cargo install cargo-deny`)
- `cargo machete` (`cargo install cargo-machete`)

## Steps

### 1. Clippy strict mode

```bash
cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic -W clippy::nursery
```

### 2. Format check

```bash
cargo fmt --all -- --check
```

### 3. Security audit

```bash
cargo audit
```

### 4. Supply chain check

```bash
cargo deny check
```

### 5. Unused dependencies

```bash
cargo machete
```

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
