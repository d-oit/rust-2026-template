# ADR 0001: Rust Edition and Toolchain

**Date**: 2025-01-01  
**Status**: Accepted (updated 2026-11-16: development toolchain uses stable; MSRV remains 1.88)
**Deciders**: Template maintainers

## Context

Every Rust project needs to decide which edition and toolchain channel to use.
The choice affects language features, compilation behavior, and ecosystem compatibility.

## Decision

Use **Rust 2024 edition** with the current **stable development toolchain** via `rust-toolchain.toml`. Declare the minimum supported Rust version independently in `Cargo.toml`.

## Rationale

### Rust 2024 Edition

- Latest edition with improved ergonomics (async closures, `impl Trait` improvements)
- Stricter lifetime rules catch bugs earlier
- Aligns with community best practices going forward

### Stable Development Toolchain

- `rust-toolchain.toml` is automatically respected by `rustup`
- Development benefits from current stable Rust without changing the project's MSRV
- CI's MSRV audit separately verifies that dependencies and workspace code support Rust 1.88
- nightly features are unstable; stable provides a guarantee

### Why not nightly?

- Nightly breaks periodically
- Production code needs stability
- If nightly is needed for specific features, it should be explicitly scoped

## Consequences

### Positive

- New development toolchains do not implicitly raise the supported MSRV
- A clear MSRV declaration in `Cargo.toml` is checked independently in CI

### Negative

- Toolchain versions can differ between local development environments as stable advances
- Some cutting-edge nightly features unavailable

## Alternatives Considered

| Option | Reason Rejected |
|--------|----------------|
| Rust 2021 edition | Less ergonomic than 2024, no benefit for new projects |
| Nightly channel | Too unstable for a template |
| Pin development to the MSRV | Prevents use of current stable features during development |

## References

- [Rust Edition Guide](https://doc.rust-lang.org/edition-guide/)
- [rust-toolchain.toml format](https://rust-lang.github.io/rustup/overrides.html#the-toolchain-file)
