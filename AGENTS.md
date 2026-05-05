# Agent Coding Guidelines

> **Canonical Source of Truth**: This file is the primary instruction set for all AI agents.
> Agent-specific files (CLAUDE.md, GEMINI.md, QWEN.md, .cursor/rules.md) are thin references only.

## Quick Reference

| Task | Command |
|------|----------|
| Build | `cargo build --workspace` |
| Format/Lint | `./scripts/code-quality.sh fmt\|clippy\|audit\|check\|fix` |
| Tests | `cargo nextest run --workspace` |
| Quality Gates | `./scripts/quality-gates.sh [--fix]` |
| Release | `./scripts/release-manager.sh` |

## Project Structure

```text
.
├── .agents/skills/      # AI agent skill definitions (runbooks)
├── .cargo/config.toml   # Cargo linker + profile + aliases
├── .config/nextest.toml # nextest profiles (default, ci)
├── .github/workflows/   # CI/CD GitHub Actions
├── crates/              # Workspace crates (libraries and apps)
├── scripts/             # Development and quality gate scripts
├── plans/adr/           # Architecture Decision Records
├── AGENTS.md            # THIS FILE (Primary guidance)
├── CLAUDE.md            # Reference to AGENTS.md
├── GEMINI.md            # Reference to AGENTS.md
└── Cargo.toml           # Workspace manifest
```

## Skill Inventory (`.agents/skills/`)

Always prefer using the defined skills for their respective tasks:

- `build-rust`: Build Rust projects correctly.
- `lint-rust`: Run clippy, formatting, and quality gate checks.
- `test-rust`: Run tests with `cargo-nextest`.
- `release-rust`: Handle the release workflow.
- `crates-io-name-check`: Verify crate name availability.
- `anti-ai-slop`: Audit and fix generic AI-generated Rust patterns.
- `privacy-first`: Prevent PII leaks in the codebase.
- `skill-creator`: Create and optimize new agent skills.
- `skill-evaluator`: Evaluate skill quality and structure.

## Core Directives

1. **Prioritize Skills & Scripts**: 1) Check `.agents/skills/` 2) Check `scripts/` 3) Use standard CLI.
2. **Zero Clippy Warnings**: All code must pass clippy without warnings. Fix issues rather than suppressing them.
3. **Async First**: Use Tokio for async logic. Avoid blocking calls in async contexts.
4. **Testing Excellence**:
   - Use `proptest` for pure functions.
   - Use `tokio::test` for async functions.
   - Target 80%+ test coverage.
5. **Code Quality**:
   - Max 500 LOC per source file.
   - Use `thiserror` for libraries and `anyhow` for binaries.
   - No `unwrap()` in library code.
6. **Documentation**: All public items must have `///` doc comments.

## Change Workflow

1. **Explore**: Read existing patterns and identify the target module.
2. **Test First**: Add or update tests before modifying implementation (TDD).
3. **Format & Lint**: Run `./scripts/code-quality.sh fix` to apply formatting and fixes.
4. **Verify**:
   - `cargo nextest run -p <package>`
   - `cargo nextest run --workspace`
   - `./scripts/quality-gates.sh`
5. **Commit**: Use Conventional Commits (`feat(scope): ...`, `fix(scope): ...`).

## Token Efficiency

1. **Discovery**: Use `list_files` or globbing.
2. **Search**: Use `grep` via bash.
3. **Inspection**: Use `read_file` for targeted inspection.
4. **Execution**: Prefer optimized scripts in `scripts/` over raw bash chains.

## Cross-References

Detailed guidance is available in `agents-docs/`:
- `agents-docs/commands.md`: Full CLI reference and aliases.
- `agents-docs/conventions.md`: Coding standards and invariants.
- `agents-docs/structure.md`: Deep dive into project layout.
- `agents-docs/workflow.md`: Step-by-step development process.
