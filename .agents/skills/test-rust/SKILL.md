---
name: test-rust
description: >
  Run the complete test suite for a Rust project including unit tests,
  integration tests, doc tests, and coverage. Use nextest for parallel execution.
  Use when running tests, debugging failures, or checking coverage.
  Triggers: "run tests", "cargo test", "test suite", "coverage".
category: rust
license: MIT
metadata:
  author: d-oit
  version: "1.0"
  tags: rust test nextest coverage proptest
---

# Skill: test-rust

## When to Use

- User asks for this skill's functionality


## Purpose

Run the complete test suite for a Rust project using best practices.

## Prerequisites

- `cargo nextest` (`cargo install cargo-nextest`)
- `cargo llvm-cov` for coverage (`cargo install cargo-llvm-cov`)

## Steps

### 1. Unit + integration tests

```bash
cargo nextest run --all-features --workspace
```

### 2. Doc tests

```bash
cargo test --doc --all-features
```

### 3. With coverage

```bash
cargo llvm-cov nextest --all-features --workspace --html
```

### 4. Report metrics

Call `metrics-reporter` skill.

## Test Organization

```
src/lib.rs          # #[cfg(test)] mod tests {}
tests/              # Integration tests
benches/            # Criterion benchmarks
```

## Success Criteria

- All tests pass
- Doc tests pass
- Coverage >= 80%

## Related Skills

- `build-rust`, `lint-rust`, `release-rust`, `metrics-reporter`

## Rationalizations

| Rationalization | Reality |
|-----------------|---------|
| "Unit tests pass, so the code is correct." | Unit tests don't catch integration issues. Run doc tests and integration tests too. |
| "Coverage is at 50%, that's probably fine." | Below 80% coverage leaves significant untested code paths. |
| "I'll skip doc tests — they're just examples." | Doc tests are compiled and run. They catch stale documentation and API mismatches. |

## Red Flags

- [ ] Running `cargo test` without `--all-features` (hides feature-gated test failures)
- [ ] Skipping doc tests (`cargo test --doc`)
- [ ] Not using `cargo-nextest` for parallel test execution

## References

- [cargo-nextest](https://nexte.st/)
- [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov)
