---
name: build-rust
description: >
  Compile, test, and verify Rust projects following 2026 best practices.
  Use when asked to build, compile, or check a Rust project, when CI/CD pipeline
  needs build steps, or when investigating build failures.
  Triggers: "build rust", "compile project", "cargo build", "check rust".
category: rust
license: MIT
metadata:
  author: d-oit
  version: "1.0"
  tags: rust build cargo compile ci
---

# Skill: build-rust

## When to Use

- User asks for this skill's functionality

## Purpose

Compile, test, and verify Rust projects following 2026 best practices.

## Trigger Conditions

- When asked to build, compile, or check a Rust project
- When CI/CD pipeline needs to run build steps
- When investigating build failures or warnings

## Prerequisites

- Rust toolchain installed (see `rust-toolchain.toml`)
- `cargo` available in PATH
- WSL2 environment (optional but recommended on Windows)

## Steps

### 1. Check (fast feedback)

```bash
cargo check --all-targets --all-features 2>&1
```

### 2. Build (debug)

```bash
cargo build --all-targets 2>&1
```

### 3. Build (release)

```bash
cargo build --release 2>&1
```

### 4. Lint with Clippy

```bash
cargo clippy --all-targets --all-features -- \
  -D warnings \
  -W clippy::pedantic \
  -W clippy::nursery \
  -A clippy::module_name_repetitions
```

### 5. Format check

```bash
cargo fmt --all -- --check
```

### 6. Run tests (with nextest)

```bash
cargo nextest run --all-features
```

### 7. Doc tests

```bash
cargo test --doc --all-features
```

### 8. Report metrics

Call `metrics-reporter` skill.

## Success Criteria

- `cargo check` exits 0 with no errors
- `cargo clippy` exits 0 with `-D warnings`
- `cargo fmt --check` exits 0
- All tests pass
- No `unsafe` blocks without `// SAFETY:` comments

## Common Issues

### Slow incremental builds in WSL2

**Symptom**: Build takes >30s even for small changes
**Fix**: Ensure `target/` is on the Linux filesystem, not Windows mount.
See `.cargo/config.toml` for WSL2 disk optimization.

### VS Code reloads during build

**Symptom**: VS Code refreshes while `cargo build` runs
**Fix**: Set `files.watcherExclude` for `**/target/**` in `.vscode/settings.json`

### Linker errors on WSL2

**Symptom**: `error: linker 'cc' not found`
**Fix**: `sudo apt-get install build-essential`

## Rationalizations

| Rationalization | Reality |
|-----------------|---------|
| "cargo build passes, so the code is correct." | Build success doesn't mean tests pass or lints are clean. Run the full pipeline. |
| "I'll skip clippy pedantic lints to save time." | Pedantic lints catch real issues. Run them in CI at minimum. |
| "Just use cargo test, nextest isn't installed." | Install nextest for parallel test execution — it's significantly faster. |

## Red Flags

- [ ] Skipping `--all-features` in clippy/test commands (hides feature-gated bugs)
- [ ] Using `cargo build` without `--all-targets` (misses test/bench compilation errors)
- [ ] Running `cargo fmt` without `--check` in CI (format drift goes unnoticed)

## Related Skills

- `test-rust` - Running the test suite
- `lint-rust` - Deep lint analysis
- `release-rust` - Creating releases
- `metrics-reporter` - Mandatory task completion reporting

## References

- [The Cargo Book](https://doc.rust-lang.org/cargo/)
- [Clippy Documentation](https://doc.rust-lang.org/clippy/)
- [cargo-nextest](https://nexte.st/)
