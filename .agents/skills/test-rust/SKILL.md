# Skill: test-rust

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

- `build-rust`, `lint-rust`, `release-rust`

## References

- [cargo-nextest](https://nexte.st/)
- [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov)
