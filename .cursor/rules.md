# AI Agent Rules for Rust Workspace

@AGENTS.md

## General Principles

- Prefer `anyhow` for applications and `thiserror` for library error handling.
- Use `tokio` for asynchronous programming.
- Follow `clippy` pedantic rules and maintain a zero-warning policy.
- Adhere to the workspace/crate structure (`crates/` for members).

## Error Handling

- Use `thiserror` for defining custom error types in libraries (`crates/example-crate`).
- Use `anyhow` in binaries (`crates/sample-app`) for high-level error context and management.
- Avoid `unwrap()` and `expect()` in library code.

## Asynchronous Programming

- Use `#[tokio::main]` for entry points.
- Prefer `tokio` primitives for synchronization and I/O.
- Be mindful of blocking operations in async contexts; use `spawn_blocking` when necessary.

## Code Quality

- All code must be formatted with `cargo fmt`.
- Address all Clippy suggestions.
- Write unit tests for new functionality.
- Use `proptest` for property-based testing of pure functions.

## Workspace Conventions

- New crates should be added to the `crates/` directory.
- Update the root `Cargo.toml` when adding workspace members.
