# Agent Coding Guidelines

> **2026 Best Practice Rust Template**  
> All AI agents (Claude, Gemini, OpenCode, Cursor, etc.) read this file first.
> CLAUDE.md and GEMINI.md both reference this file via `@AGENTS.md`.

## Quick Reference

| Task | Command |
|------|----------|
| **Build (dev)** | `cargo build --workspace` |
| **Build (release)** | `cargo build --release --workspace` |
| **Type-check only** | `cargo check --workspace` |
| **Format** | `cargo fmt --all` |
| **Lint** | `cargo clippy --all -- -D warnings` |
| **Tests** | `cargo nextest run --all` |
| **Doc tests** | `cargo test --doc` |
| **Security audit** | `cargo audit` |
| **Quality gates** | `./scripts/quality-gates.sh` |
| **Full CI local** | `./scripts/ci-local.sh` |

## Project Structure

```
rust-2026-template/
├── .agents/skills/        # AI agent skill definitions
├── .cargo/config.toml     # Cargo linker + profile config
├── .claude/               # Claude-specific config
├── .config/nextest.toml   # nextest profiles
├── .github/
│   ├── workflows/         # CI/CD GitHub Actions
│   ├── CODEOWNERS         # Code ownership
│   └── PULL_REQUEST_TEMPLATE.md
├── .githooks/             # Pre-commit hooks
├── .opencode/             # OpenCode agent config
├── .vscode/settings.json  # VS Code / WSL2 settings
├── src/                   # Main library source
├── benches/               # Criterion benchmarks
├── examples/              # Usage examples
├── tests/                 # Integration tests
├── scripts/               # Dev helper scripts
├── docs/                  # Documentation
├── plans/adr/             # Architecture Decision Records
├── AGENTS.md              # THIS FILE - AI agent instructions
├── CLAUDE.md              # Claude: @AGENTS.md
├── GEMINI.md              # Gemini: @AGENTS.md
├── Cargo.toml             # Workspace manifest
├── rust-toolchain.toml    # Pinned toolchain
├── rustfmt.toml           # Formatting config
├── .clippy.toml           # Clippy config
├── deny.toml              # cargo-deny config
├── release.toml           # cargo-release config
└── CHANGELOG.md           # Keep-a-changelog format
```

## Skill + CLI Pattern (CRITICAL)

**ALWAYS use skills first** for high-frequency operations:

| Operation | Skill | Command | Token Cost |
|-----------|-------|---------|------------|
| Build | `build-rust` | `cargo build --workspace` | Low |
| Format/Lint | `code-quality` | `./scripts/code-quality.sh` | Low |
| Quality Gates | `code-quality` | `./scripts/quality-gates.sh` | Medium |
| Tests | `test-runner` | `cargo nextest run --all` | Medium |
| Debug | `debug-troubleshoot` | `RUST_LOG=debug cargo nextest run` | Medium |
| CI Issues | `github-workflows` | `gh workflow list` | Low |
| Release | `release-guard` | `./scripts/release.sh` | High |

**Decision tree before using task tool:**
1. Is there a skill in `.agents/skills/`? → Use it
2. Is there a script in `scripts/`? → Use it
3. Is this high-frequency? → Use Skill + CLI
4. Is this complex multi-agent? → Use task tool

## Change Workflow (Golden Path)

1. Read existing code patterns before modifying
2. Identify owner module + relevant file
3. Add/update tests first (TDD preferred)
4. `cargo fmt --all` → fix formatting
5. `cargo clippy --all -- -D warnings` → fix ALL warnings
6. `cargo nextest run -p <crate>` → targeted tests
7. `cargo nextest run --all` → full suite
8. `cargo test --doc` → doc tests
9. `./scripts/quality-gates.sh` → final validation
10. Commit with conventional format

## Required Checks Before Every Commit

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all -- -D warnings`
- [ ] `cargo build --workspace`
- [ ] `cargo nextest run --all`
- [ ] `cargo test --doc`
- [ ] `./scripts/quality-gates.sh`

## Code Conventions (Non-Negotiable)

- **Max 500 LOC per source file** - split into submodules when exceeded
- **Zero clippy warnings** - fix, never suppress with `#[allow(...)]` without comment
- **Single responsibility** per module
- **Async everywhere** - Tokio runtime, no blocking in async paths (use `spawn_blocking`)
- **Error handling** - `thiserror` for library errors, `anyhow` for binaries
- **No `unwrap()`** in library code - propagate errors
- **Doc comments** on all public items (`///`)
- **Tests required** - `#[tokio::test]` for async, AAA pattern (Arrange-Act-Assert)

## Core Invariants (Never Break)

- **Async**: Tokio runtime everywhere. No `std::thread::sleep` in async paths
- **SQL**: Parameterized queries only. Short transactions. No locks across `.await`
- **Clippy**: Zero warnings enforced (`-D warnings`). Fix, don't suppress
- **Files**: ≤500 LOC per source file. Split into submodules when exceeded
- **Tests**: ≥80% coverage target. `#[tokio::test]` for async
- **Secrets**: Never hardcode. Use environment variables or `.env` files
- **Dependencies**: Review before adding. Run `cargo audit` and `cargo deny check`

## Testing Strategy (2026)

| Layer | Tool | When |
|-------|------|------|
| Unit | `cargo nextest` | Always |
| Integration | `cargo nextest` (tests/) | Always |
| Doc tests | `cargo test --doc` | Always |
| Benchmarks | `cargo bench` (Criterion) | CI nightly |
| Property | `proptest` | Core invariants |
| Snapshot | `insta` | CLI/API output |
| Mutation | `cargo-mutants` | Pre-release |
| Fuzz | `cargo-fuzz` | Security-critical |

**nextest profiles** (`.config/nextest.toml`):
- `default` - local dev, fast feedback
- `ci` - CI with retries and JUnit output
- `nightly` - mutation + fuzz + full coverage

## Disk Space Management (WSL2/Linux)

```toml
# .cargo/config.toml - keeps target/ small
[profile.dev]
debug = "line-tables-only"    # ~60% smaller

[profile.dev.package."*"]
debug = false                  # no debug info for deps

[profile.dev.build-override]
opt-level = 3                  # fast proc-macros
```

- Use `mold` linker on Linux: `RUSTFLAGS="-C link-arg=-fuse-ld=mold"`
- Exclude `target/` from VS Code watcher (see `.vscode/settings.json`)
- Run `cargo clean` or `./scripts/clean-artifacts.sh` periodically

## Feature Flags Pattern

```toml
[features]
default = []
full = ["feature-a", "feature-b"]
feature-a = ["dep:some-crate"]
feature-b = []
```

Always test with `--all-features` in CI.

## Commit Format (Conventional Commits)

```
feat(module): add new capability
fix(module): resolve bug description
docs(module): update documentation
chore(deps): update dependencies
refactor(module): restructure without behavior change
test(module): add missing tests
ci: update workflow
perf(module): improve performance
```

## Release Workflow

1. All quality gates pass
2. `cargo semver-checks check-release` (no breaking changes without major bump)
3. `cargo release patch|minor|major` (updates versions)
4. `cargo dist build` (binaries + installers)
5. Tag pushed → CI builds and publishes

See `plans/adr/` for Architecture Decision Records.

## Environment Variables

| Variable | Purpose | Required |
|----------|---------|----------|
| `RUST_LOG` | Log level (debug/info/warn/error) | No (default: info) |
| `RUST_BACKTRACE` | Backtrace on panic (1/full) | No |
| `DATABASE_URL` | Primary DB connection string | If using DB |

See `.env.example` for full list. **Never commit secrets.**

## Security

- Use environment variables (never hardcode secrets)
- Run `cargo audit` before every release
- Run `cargo deny check` in CI
- Input validation at all API boundaries
- Use `cargo-geiger` to audit unsafe code
- See `SECURITY.md` for vulnerability reporting

## Performance Targets

Document performance budgets in `plans/adr/`. Use Criterion benchmarks with baselines:

```bash
cargo bench --bench benchmark -- --save-baseline main
cargo bench --bench benchmark -- --baseline main
```

CI fails if regression > 10%.

## Cross-References

| Topic | Document |
|-------|----------|
| Architecture decisions | `plans/adr/` |
| Testing strategies | `docs/TESTING.md` |
| Code style | `docs/CODE_CONVENTIONS.md` |
| Release engineering | `docs/RELEASE.md` |
| Security policies | `SECURITY.md` |
| Contributing | `CONTRIBUTING.md` |
| Skills | `.agents/skills/` |
