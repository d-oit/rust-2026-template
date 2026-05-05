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

## Crate Naming (MANDATORY before `cargo publish`)

Before naming any new crate, **always verify the name is available on crates.io**:

```bash
# Method 1: cargo search (recommended, no browser needed)
cargo search <your-crate-name>
# If output contains exactly your name → taken. Choose another.

# Method 2: direct API check
curl -s https://crates.io/api/v1/crates/<your-crate-name> | python3 -m json.tool | grep '"name"'
# 404 response → name is available

# Method 3: browser
# https://crates.io/crates/<your-crate-name>
```

**Best practice rules for crate names:**

- Use `kebab-case` (hyphens, not underscores) for the crate name in `Cargo.toml` `[package] name`
- Keep names short, descriptive, and unambiguous
- Avoid squatting on generic names (e.g. `utils`, `helpers`, `core`)
- Prefer scoped, specific names: `myproject-core`, `myproject-cli`
- If publishing to crates.io, claim the name early with a `0.1.0` placeholder release
- Check for similar names that could cause confusion: `cargo search <prefix>`

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

## CI & Linting

- **Commit Messages**: Body lines must be ≤ 100 characters.
- **Markdown**: Fenced code blocks must have blank lines before and after.
