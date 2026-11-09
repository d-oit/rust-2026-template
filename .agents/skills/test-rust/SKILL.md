---
name: test-rust
description: >
  Run the complete test suite for a Rust project including unit tests,
  integration tests, doc tests, and coverage. Use nextest for parallel execution.
  Use when running tests, debugging failures, or checking coverage.
  Triggers: "run tests", "cargo test", "test suite", "coverage".

license: MIT
metadata:
  category: rust
  author: d-oit
  version: "1.0"
  tags: rust test nextest coverage proptest
---

# Skill: test-rust

## Purpose

Run the complete test suite for a Rust project using best practices.

## Prerequisites

- `cargo nextest` (`cargo install cargo-nextest`)
- `cargo llvm-cov` for coverage (`cargo install cargo-llvm-cov`)

## Steps

### 1. Unit + integration tests

```bash
cargo nextest run --all-features --workspace --no-tests=fail
```

For this repository, `./scripts/quality-gates.sh` is the canonical full quality
pipeline; this direct command is useful for focused test iteration.

### 2. Doc tests

```bash
cargo test --workspace --doc --all-features
```

### 3. With coverage

```bash
cargo llvm-cov nextest --all-features --workspace --html
```

### 4. Report metrics

Call `metrics-reporter` skill.

## Test Organization

```text
crates/<crate>/src/lib.rs       # Unit tests in #[cfg(test)] modules
crates/<crate>/tests/           # Crate-level integration tests
crates/workspace-tests/tests/   # Cross-crate integration and fitness tests
benchmarks/benches/             # Criterion benchmarks
```

## Success Criteria

- All tests pass
- Doc tests pass
- Coverage meets `.codecov.yml` targets: 70% project coverage and 60% patch coverage;
  project coverage may drop by no more than the configured 2% threshold

## Related Skills

- `build-rust`, `lint-rust`, `release-rust`, `metrics-reporter`

## Rationalizations

| Rationalization | Reality |
|-----------------|---------|
| "Unit tests pass, so the code is correct." | Unit tests don't catch integration issues. Run doc tests and integration tests too. |
| "Coverage is at 50%, that's probably fine." | The repository sets a 70% project target and 60% patch target in `.codecov.yml`; check the applicable target rather than assuming coverage is sufficient. |
| "I'll skip doc tests — they're just examples." | Doc tests are compiled and run. They catch stale documentation and API mismatches. |

## Red Flags

- [ ] Running `cargo test` without `--all-features` (hides feature-gated test failures)
- [ ] Skipping doc tests (`cargo test --workspace --doc --all-features`)
- [ ] Not using `cargo-nextest` for parallel test execution

## References

- [cargo-nextest](https://nexte.st/)
- [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov)
