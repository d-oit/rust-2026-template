# Agent Coding Guidelines

> **2026 Best Practice Rust Template** - All AI agents read this file first.

## Quick Reference

| Task | Command |
|------|----------|
| Build | `cargo build --workspace` |
| Quality | `./scripts/code-quality.sh fmt\|clippy\|audit\|check` |
| Tests | `cargo nextest run --all` |
| Quality Gates | `./scripts/quality-gates.sh` |

## Project Structure

```
rust-2026-template/
├── .agents/skills/      # AI agent skill definitions
├── .cargo/config.toml   # Cargo linker + profile config
├── .config/nextest.toml # nextest profiles
├── .github/workflows/   # CI/CD GitHub Actions
├── scripts/             # Dev helper scripts
├── plans/adr/           # Architecture Decision Records
├── AGENTS.md            # THIS FILE
├── CLAUDE.md            # Claude: @AGENTS.md
├── GEMINI.md            # Gemini: @AGENTS.md
└── Cargo.toml           # Workspace manifest
```

## Skill + CLI Pattern (CRITICAL)

**ALWAYS use Skill + CLI first** for high-frequency ops:

| Operation | Skill | Script/CLI |
|-----------|-------|------------|
| Build | `build-rust` | `./scripts/build-rust.sh` |
| Format/Lint | `code-quality` | `./scripts/code-quality.sh` |
| Quality Gates | `code-quality` | `./scripts/quality-gates.sh` |
| Tests | `test-runner` | `cargo nextest run --all` |

**Before task tool:** 1) Skill in `.agents/skills/`? → Use it 2) Script in `scripts/`? → Use it 3) High-frequency? → Skill+CLI 4) Complex multi-agent? → task tool

## Token Efficiency

1. **Glob** - File discovery (cheapest)
2. **Grep** - Code search (cheap)
3. **Read** - File inspection (medium)
4. **Bash** - Shell commands (expensive - prefer scripts)

## Change Workflow

1. Read existing code patterns
2. Identify owner module + file
3. Add/update tests first (TDD)
4. `./scripts/code-quality.sh fmt`
5. `cargo clippy --all -- -D warnings`
6. `cargo nextest run -p <package>`
7. `cargo nextest run --all`
8. `./scripts/quality-gates.sh`
9. Commit with conventional format

## Required Checks Before Commit

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --tests -- -D warnings`
- [ ] `cargo build --workspace`
- [ ] `cargo nextest run --all`
- [ ] `./scripts/quality-gates.sh`

## Code Conventions

- **Max 500 LOC** per source file - split when exceeded
- **Zero clippy warnings** - fix, never suppress
- **Async everywhere** - Tokio, no blocking in async paths
- **Testing** - Use `proptest` for pure functions; `tokio::test` for async
- **Error handling** - `thiserror` lib, `anyhow` bin
- **No `unwrap()`** in library code
- **Doc comments** on all public items (`///`)
- **Tests** - `#[tokio::test]` for async, AAA pattern

## Core Invariants

- **Async**: Tokio runtime, no blocking (use `spawn_blocking`)
- **Clippy**: Zero warnings (`-D warnings`)
- **Files**: ≤500 LOC
- **Tests**: ≥80% coverage, `#[tokio::test]`
- **Secrets**: Never hardcode, use `.env` or better env vars

## Cross-References

| Topic | Document |
|-------|----------|
| Commands | `agents-docs/commands.md` |
| Structure | `agents-docs/structure.md` |
| Conventions | `agents-docs/conventions.md` |
| Workflow | `agents-docs/workflow.md` |
| Architecture | `plans/adr/` |
| Skills | `.agents/skills/` |
