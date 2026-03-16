# Code Conventions

## Non-Negotiable Rules

- **Max 500 LOC per source file** - split into submodules when exceeded
- **Zero clippy warnings** - fix, never suppress with `#[allow(...)]` without comment
- **Single responsibility** per module
- **Async everywhere** - Tokio runtime, no blocking in async paths
- **Error handling** - `thiserror` for library errors, `anyhow` for binaries
- **No `unwrap()`** in library code - propagate errors
- **Doc comments** on all public items (`///`)
- **Tests required** - `#[tokio::test]` for async, AAA pattern

## Core Invariants

- **Async**: Tokio runtime everywhere. No blocking in async paths (use `spawn_blocking`)
- **Clippy**: Zero warnings enforced (`-D warnings`). Fix, don't suppress
- **Files**: ≤500 LOC per source file
- **Tests**: ≥80% coverage target. `#[tokio::test]` for async
- **Secrets**: Never hardcode. Use environment variables or `.env` files

## Testing Strategy

| Layer | Tool | When |
|-------|------|------|
| Unit/Integration | `cargo nextest` | Always |
| Doc tests | `cargo test --doc` | Always |
| Property | `proptest` | Core invariants |
| Snapshot | `insta` | CLI/API output |
| Mutation | `cargo-mutants` | Pre-release |

## Disk Space (WSL2/Linux)

```toml
# .cargo/config.toml
[profile.dev]
debug = "line-tables-only"
[profile.dev.package."*"]
debug = false
```

## Commit Format

`feat(module): description`
`fix(module): description`
`chore(deps): update dependencies`