# Commands Reference

## Build & Quality

| Task | Command |
|------|----------|
| Build (dev) | `cargo build --workspace` |
| Format | `./scripts/code-quality.sh fmt` |
| Clippy | `./scripts/code-quality.sh clippy` |
| Audit | `./scripts/code-quality.sh audit` |
| Check | `./scripts/code-quality.sh check` |
| Tests | `cargo nextest run --all` |
| Doc tests | `cargo test --doc` |
| Quality Gates | `./scripts/quality-gates.sh` |
| Release validate | `./scripts/release-manager.sh validate` |

## CI Parity

| Check | Local Command |
|-------|---------------|
| Full CI Parity | `./scripts/code-quality.sh check` |
| Clippy (tests) | `./scripts/code-quality.sh clippy` |

## Release Workflow

```bash
./scripts/quality-gates.sh
cargo semver-checks check-release
cargo release [patch|minor|major]
./scripts/release-manager.sh --execute
```
