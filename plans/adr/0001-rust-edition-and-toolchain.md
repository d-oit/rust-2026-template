# ADR 0001: Rust Edition and Toolchain

**Date**: 2025-01-01  
**Status**: Accepted  
**Deciders**: Template maintainers

## Context

Every Rust project needs to decide which edition and toolchain channel to use.
The choice affects language features, compilation behavior, and ecosystem compatibility.

## Decision

Use **Rust 2024 edition** with a **pinned stable toolchain** (1.85+) via `rust-toolchain.toml`.

## Rationale

### Rust 2024 Edition
- Latest edition with improved ergonomics (async closures, `impl Trait` improvements)
- Stricter lifetime rules catch bugs earlier
- Aligns with community best practices going forward

### Pinned Stable Toolchain
- Reproducible builds across all environments
- `rust-toolchain.toml` is automatically respected by `rustup`
- Avoids "works on my machine" issues from toolchain drift
- nightly features are unstable; stable provides a guarantee

### Why not nightly?
- Nightly breaks periodically
- Production code needs stability
- If nightly is needed for specific features, it should be explicitly scoped

## Consequences

### Positive
- Reproducible, deterministic builds
- Clear upgrade path (bump version in `rust-toolchain.toml`)
- CI and local environments stay in sync

### Negative
- Must update `rust-toolchain.toml` manually for new Rust versions
- Some cutting-edge nightly features unavailable

## Alternatives Considered

| Option | Reason Rejected |
|--------|----------------|
| Rust 2021 edition | Less ergonomic than 2024, no benefit for new projects |
| Nightly channel | Too unstable for a template |
| No toolchain pin | Leads to inconsistent environments |

## References
- [Rust Edition Guide](https://doc.rust-lang.org/edition-guide/)
- [rust-toolchain.toml format](https://rust-lang.github.io/rustup/overrides.html#the-toolchain-file)
