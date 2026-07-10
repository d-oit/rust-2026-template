# Lessons Learned

## Project Evolution

### Architecture Decisions

1. **Single Source of Truth (ADR-0001)**: `AGENTS.md` is the canonical instruction file. Tool-specific files (`CLAUDE.md`, `GEMINI.md`, etc.) contain only thin adapters with `@AGENTS.md` references. This prevents instruction drift across agent platforms.

1. **CI Path Filtering (ADR-0002)**: Using `dorny/paths-filter` to detect changed files before running expensive CI jobs. This reduces CI time by ~60% for documentation-only changes.

1. **Template Crate Patterns (ADR-0003)**: Workspace member crates demonstrate different architectural patterns (actor model, storage, MCP server, checkpoint). Each pattern is self-contained with its own README.

### Quality Gates

1. **11-Stage Quality Gate**: The quality gate script auto-detects languages and runs appropriate checks. Adding a new language requires only adding detection logic and check commands.

1. **Pre-commit + CI Defense in Depth**: Pre-commit hooks catch issues locally; CI catches what slips through. Both run the same underlying checks but in different environments.

1. **LOC Limit Enforcement**: 500 LOC per source file prevents mega-files. Enforcement happens at pre-commit (git hook) and CI (quality gate).

### Agent Patterns

1. **Skills over Monoliths**: Breaking agent knowledge into discrete skills (18 total) enables reuse and composition. Each skill is self-contained with its own evals.

1. **Event-Based Metrics**: Writing immutable JSON event files to `.agents/events/YYYY/MM/DD/` prevents merge conflicts and enables historical analysis.

1. **Session Bootstrap**: Auto-injecting project context at session start reduces agent orientation time from minutes to seconds.

### Rust-Specific

1. **Workspace Version Propagation**: Using `version.workspace = true` in member crates ensures version consistency. The `propagate-version.sh` script automates updates.

1. **cargo-deny for Supply Chain**: `deny.toml` enforces license allowlists, bans problematic crates, and checks for known vulnerabilities. Runs in both pre-commit and CI.

1. **nextest over cargo test**: Parallel test execution with `cargo-nextest` reduces test time by ~40%. Profile configurations in `.config/nextest.toml`.

### CI/CD

1. **SHA-Pinned Actions**: All GitHub Actions are pinned to commit SHAs, not tags. This prevents supply-chain attacks via tag mutation.

1. **DORA Metrics Integration**: Automated tracking of Deployment Frequency, Lead Time, Change Failure Rate, and Failed Deployment Recovery Time via workflow events.

1. **Self-Learning Labels**: `learn-labels.sh` mines commits, issues, and PRs to discover missing labels. The label set evolves with the project.

## Common Pitfalls

### Don'ts

- **Don't** duplicate instructions across tool-specific files. Use `@AGENTS.md` references.
- **Don't** skip the quality gate. It's designed to catch issues early.
- **Don't** use `unwrap()` in library code. Use `thiserror` for typed errors.
- **Don't** commit without running pre-commit hooks. They exist for a reason.
- **Don't** add dependencies without checking `cargo-deny` configuration.

### Do's

- **Do** run `./scripts/doctor.sh` when something seems wrong.
- **Do** check `.agents/ci/ci-status.json` before proposing changes.
- **Do** write ADRs for architectural decisions.
- **Do** use conventional commits for changelog generation.
- **Do** test skills with `./scripts/run-evals.sh` before merging.

## Performance Insights

### Build Times

- **Mold linker**: optional on Linux; entries in `.cargo/config.toml` are commented out by default for portability (codespaces without mold). Uncomment after installing mold. On Rust ≥1.90, `rust-lld` is the default linker on `x86_64-unknown-linux-gnu` without extra config. https://blog.rust-lang.org/2025/09/01/rust-lld-on-1.90.0-stable/
- **sccache**: Caches compilation across builds (enabled in CI via `RUSTC_WRAPPER`)
- **Workspace incremental**: Cargo's workspace-level incremental compilation

### Test Times

- **nextest**: ~40% faster than `cargo test` via parallel execution
- **Test profiles**: CI profile with retries and slow-timeout in `.config/nextest.toml`
- **Doc tests**: Run separately to avoid blocking unit test parallelism

### CI Times

- **Path filtering**: ~60% reduction for docs-only changes
- **Concurrent jobs**: Independent lint/test/security jobs run in parallel
- **Caching**: `actions/cache` for cargo registry and build artifacts
