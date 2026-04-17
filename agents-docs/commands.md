# Commands Reference

## Build & Quality

| Task | Command |
|------|----------|
| Build (dev) | `cargo build --workspace` |
| Build (release) | `cargo build --workspace --release` |
| Format | `./scripts/code-quality.sh fmt` |
| Clippy | `./scripts/code-quality.sh clippy` |
| Audit | `./scripts/code-quality.sh audit` |
| Full CI parity | `./scripts/code-quality.sh check` |
| Auto-fix | `./scripts/code-quality.sh fix` |
| Tests | `cargo nextest run --workspace` |
| Doc tests | `cargo test --doc` |
| Quality Gates (all) | `./scripts/quality-gates.sh` |
| Quality Gates (fix) | `./scripts/quality-gates.sh --fix` |

## Cargo Aliases (`.cargo/config.toml`)

| Alias | Expands to |
|-------|-----------|
| `cargo check-all` | `cargo check --workspace --all-features` |
| `cargo test-all` | `cargo nextest run --workspace` |
| `cargo fmt-check` | `cargo fmt --all -- --check` |
| `cargo lint` | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| `cargo audit-check` | `cargo deny check` |
| `cargo release-check` | `cargo semver-checks check-release` |

## Targeted Commands

```bash
# Run tests for a single crate
cargo nextest run -p example-crate
cargo nextest run -p sample-app

# Run the sample app
cargo run -p sample-app
cargo run -p sample-app -- --count 5 --verbose
cargo run -p sample-app -- --config config.json
```

## CI Parity

| CI Job | Local Equivalent |
|--------|-----------------|
| Format | `cargo fmt --all -- --check` |
| Clippy | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| Test | `cargo nextest run --workspace --all-features --profile ci` |
| Doc tests | `cargo test --workspace --doc` |
| Security | `cargo audit` |
| Deny | `cargo deny check` |
| MSRV | `rustup run 1.87 cargo check --workspace` |

## Release Workflow

```bash
# 1. Validate (dry-run)
./scripts/release-manager.sh validate

# 2. Prepare version bump (dry-run)
./scripts/release-manager.sh prepare patch

# 3. Execute release
./scripts/release-manager.sh validate --execute
cargo release [patch|minor|major]
```
