# Agent Coding Guidelines

> **2026 Best Practice Rust Template**
> All AI agents (Claude, Gemini, OpenCode, Cursor, etc.) read this file first.
> CLAUDE.md and GEMINI.md both reference this file via `@AGENTS.md`.

## Quick Reference

| Task | Command |
|------|----------|
| **Build (dev)** | `cargo build --workspace` |
| **Quality** | `./scripts/code-quality.sh fmt|clippy|audit|check` |
| **Tests** | `cargo nextest run --all` (doctests: `cargo test --doc`) |
| **Quality Gates** | `./scripts/quality-gates.sh` |
| **Docs Integrity** | `./scripts/check-docs-integrity.sh` |
| **Release Ops** | `./scripts/release-manager.sh validate|prepare|publish|full` |

## Project Structure

```
rust-2026-template/
├── .agents/skills/      # AI agent skill definitions
├── .cargo/config.toml   # Cargo linker + profile config
├── .claude/             # Claude-specific config
├── .config/nextest.toml # nextest profiles
├── .github/
│   ├── workflows/       # CI/CD GitHub Actions
│   └── PULL_REQUEST_TEMPLATE.md
├── .vscode/settings.json # VS Code / WSL2 settings
├── scripts/             # Dev helper scripts
├── plans/adr/           # Architecture Decision Records
├── AGENTS.md            # THIS FILE - AI agent instructions
├── CLAUDE.md            # Claude: @AGENTS.md
├── GEMINI.md            # Gemini: @AGENTS.md
└── Cargo.toml           # Workspace manifest
```

## Skill + CLI Pattern (CRITICAL)

**ALWAYS use Skill + CLI first** for high-frequency operations:

| Operation | Skill | Script/CLI | Token Cost |
|-----------|-------|-------------|-------------|
| Build | `build-rust` | `./scripts/build-rust.sh` | Low |
| Format/Lint | `code-quality` | `./scripts/code-quality.sh` | Low |
| Quality Gates | `code-quality` | `./scripts/quality-gates.sh` | Medium |
| CI Issues | `github-workflows` | `gh workflow list` | Low |
| Tests | `test-runner` | `cargo nextest run --all` | Medium |
| Debug | `debug-troubleshoot` | `RUST_LOG=debug cargo nextest run` | Medium |

**Before using task tool:**
1. Is there a skill in `.agents/skills/`? → Use it
2. Is there a script in `scripts/`? → Use it
3. Is this high-frequency? → Use Skill + CLI
4. Is this complex multi-agent? → Use task tool

## Token Efficiency (2026-03)

**Tool Selection Priority (lowest token cost first):**
1. **Glob** - File discovery (cheapest, structured output)
2. **Grep** - Code search (cheap, file-by-file breakdown)
3. **Read** - File inspection (medium)
4. **Bash** - Shell commands (expensive - prefer scripts)

**Verified Patterns:**
- Grep tool: 1 call → structured file-by-file breakdown
- Glob tool: 1 call → all matching files with paths
- Scripts: 1 call → multiple operations combined

## Change Workflow (Golden Path)

1. **Read** existing code patterns before modifying
2. **Identify** owner module + relevant file
3. **Add/update tests** first (TDD preferred)
4. `./scripts/code-quality.sh fmt` → fix formatting
5. `cargo clippy --all -- -D warnings` → fix ALL warnings
6. `cargo nextest run -p <package>` → targeted tests
7. `cargo nextest run --all` → full suite
8. `./scripts/quality-gates.sh` → final validation
9. **Commit** with conventional format

## Required Checks Before Every Commit

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --tests -- -D warnings` (CI parity)
- [ ] `cargo build --workspace`
- [ ] `cargo nextest run --all`
- [ ] `./scripts/quality-gates.sh`

## CI Parity (2026-03)

**CRITICAL**: Local checks must match CI exactly to prevent "works locally, fails in CI".

| Check | Local Command |
|-------|---------------|
| Full CI Parity | `./scripts/code-quality.sh check` |
| Clippy (tests) | `./scripts/code-quality.sh clippy` |

## Code Conventions (Non-Negotiable)

- **Max 500 LOC per source file** - split into submodules when exceeded
- **Zero clippy warnings** - fix, never suppress with `#[allow(...)]` without comment
- **Single responsibility** per module
- **Async everywhere** - Tokio runtime, no blocking in async paths
- **Error handling** - `thiserror` for library errors, `anyhow` for binaries
- **No `unwrap()`** in library code - propagate errors
- **Doc comments** on all public items (`///`)
- **Tests required** - `#[tokio::test]` for async, AAA pattern

## Core Invariants (Never Break)

- **Async**: Tokio runtime everywhere. No blocking in async paths (use `spawn_blocking`)
- **Clippy**: Zero warnings enforced (`-D warnings`). Fix, don't suppress
- **Files**: ≤500 LOC per source file.
- **Tests**: ≥80% coverage target. `#[tokio::test]` for async
- **Secrets**: Never hardcode. Use environment variables or `.env` files

## Testing Strategy (2026)

| Layer | Tool | When |
|-------|------|------|
| Unit/Integration | `cargo nextest` | Always |
| Doc tests | `cargo test --doc` | Always |
| Property | `proptest` | Core invariants |
| Snapshot | `insta` | CLI/API output |
| Mutation | `cargo-mutants` | Pre-release |

## Disk Space Management (WSL2/Linux)

```toml
# .cargo/config.toml - keeps target/ small
[profile.dev]
debug = "line-tables-only" # ~60% smaller
[profile.dev.package."*"]
debug = false              # no debug info for deps
```

## Commit Format (Conventional Commits)

`feat(module): description`
`fix(module): description`
`chore(deps): update dependencies`

## Self-Learning Patterns (2026-03)

1. **Systematic codebase analysis** before planning.
2. **Write ADRs** for every non-trivial architectural change before implementation.
3. **Add executable scripts to skills** — agent can run them directly.
4. **Treat CI failures as blockers** — verify empty required-check rollup as a failure evidence.

## Release Workflow

1. All quality gates pass (`./scripts/quality-gates.sh`)
2. `cargo semver-checks check-release`
3. `cargo release [patch|minor|major]`
4. `./scripts/release-manager.sh --execute`

## Cross-References

| Topic | Document |
|-------|----------|
| Architecture decisions | `plans/adr/` |
| Testing strategies | `docs/TESTING.md` |
| Release engineering | `docs/RELEASE.md` |
| Skills | `.agents/skills/` |
