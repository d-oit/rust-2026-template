# Skill: lint-rust

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

## References
- [Clippy lints](https://rust-lang.github.io/rust-clippy/master/)
- [cargo-audit](https://crates.io/crates/cargo-audit)
- [cargo-deny](https://embarkstudios.github.io/cargo-deny/)
