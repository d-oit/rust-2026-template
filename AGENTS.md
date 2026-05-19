# Agent Coding Guidelines

> **2026 Best Practice Rust Template** - This is the single canonical instruction file for all AI agents.
> Root-level agent files (`CLAUDE.md`, `GEMINI.md`, `QWEN.md`, etc.) are thin wrappers that point here.

## Quick Reference

| Task | Command |
|------|----------|
| Build | `cargo build --workspace` |
| Quality | `./scripts/code-quality.sh fmt\|clippy\|audit\|check` |
| Tests | `cargo nextest run --workspace` |
| Quality Gates | `./scripts/quality-gates.sh` |

## Project Structure

```text
.
├── .agents/skills/      # AI agent skill definitions (canonical workflows)
├── .cargo/config.toml   # Cargo linker + profile config
├── .config/nextest.toml # nextest profiles
├── .github/workflows/   # CI/CD GitHub Actions
├── scripts/             # Development and quality scripts
├── plans/adr/           # Architecture Decision Records
├── AGENTS.md            # THIS FILE (Canonical Guidance)
├── CLAUDE.md            # Claude-specific reference (@AGENTS.md)
├── GEMINI.md            # Gemini-specific reference (@AGENTS.md)
├── QWEN.md              # Qwen-specific reference (@AGENTS.md)
└── Cargo.toml           # Workspace manifest
```

## Agent Skills (.agents/skills/)

Specialized workflows are defined as "skills". Always consult the relevant skill's `SKILL.md` for detailed procedures.

| Skill | Purpose |
|-------|---------|
| `build-rust` | Optimized build procedures |
| `lint-rust` | Formatting and static analysis workflows |
| `test-rust` | Comprehensive testing with `nextest` and `proptest` |
| `release-rust` | Safe crate release process |
| `anti-ai-slop` | Auditing and fixing generic AI code patterns |
| `privacy-first` | PII and data leakage prevention |
| `crates-io-name-check` | Registry availability verification |
| `skill-creator` | Guidelines for creating new agent skills |
| `skill-evaluator` | Quality assessment of existing skills |

## Change Workflow

1. **Discover:** Read existing code patterns and module structure.
2. **Plan:** Identify affected files and required test coverage.
3. **Test-First:** Add or update tests before implementing logic (TDD).
4. **Implement:** Write code adhering to project conventions.
5. **Quality Check:**
   - `./scripts/code-quality.sh fmt`
   - `./scripts/code-quality.sh clippy`
   - `cargo nextest run --workspace`
   - `./scripts/quality-gates.sh` (Final local validation)
6. **Commit:** Use conventional commit format (e.g., `feat: ...`, `fix: ...`).

## Required Checks Before Submit

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --tests -- -D warnings`
- [ ] `cargo build --workspace`
- [ ] `cargo nextest run --workspace`
- [ ] `./scripts/quality-gates.sh`

## Code Conventions

- **File Size:** Max 500 LOC per source file; refactor into submodules if exceeded.
- **Lints:** Zero `clippy` warnings allowed. Do not suppress warnings without extreme justification.
- **Concurrency:** Prefer `tokio` for async logic. Avoid blocking calls in async contexts.
- **Error Handling:** Use `thiserror` for libraries and `anyhow` for applications/binaries.
- **Safety:** `#![forbid(unsafe_code)]` is strictly enforced. No `unwrap()` in library code.
- **Documentation:** All public items must have `///` doc comments.
- **Testing:** Use `proptest` for pure functions and `tokio::test` for async logic.

## Core Invariants

- **Performance:** Use `mold` linker and optimized dev profiles (see `.cargo/config.toml`).
- **Security:** Never hardcode secrets; use environment variables or a `.env` file.
- **Privacy:** Adhere to the `privacy-first` skill to avoid leaking PII.

## Test Quality

The project enforces automated test quality thresholds defined in `.test-quality.toml`.

- **Coverage:** Minimum 70% line coverage and 60% patch coverage.
- **Ratio:** Minimum 0.5 test-to-source LOC ratio (verified via `scripts/quality-gates.sh`).
- **Minimum Tests:** At least 20 test functions must exist across the workspace.
- **Mutation:** Minimum 60% mutation score (verified via `cargo-mutants` in CI).

Agents should ensure that any new feature includes sufficient tests to maintain or improve these ratios.

## Cross-References

| Topic | Document |
|-------|----------|
| Detailed Commands | `agents-docs/commands.md` |
| Code Structure | `agents-docs/structure.md` |
| Coding Conventions | `agents-docs/conventions.md` |
| Workflow Details | `agents-docs/workflow.md` |
| Architecture | `plans/adr/` |
| Skills Directory | `.agents/skills/` |
