# OpenCode Rules

Refer to **[AGENTS.md](AGENTS.md)** for the canonical instruction set for this repository.

## Project Principles

- Prefer `anyhow` for applications and `thiserror` for library error handling.
- Use `tokio` for asynchronous programming.
- Follow `clippy` pedantic rules and maintain a zero-warning policy.
- Adhere to the workspace/crate structure (`crates/` for members).
