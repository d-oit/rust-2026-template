# Agent Coding Contract

> **2026 Best Practice Rust Template** - This is the single canonical instruction file for all AI agents.
> Tool-specific files (`CLAUDE.md`, `.cursorrules`, etc.) are thin adapters that point here.

## Quick Reference

| Task | Command |
|------|----------|
| Build | `cargo build --workspace` |
| Quality | `./scripts/code-quality.sh fmt\|clippy\|audit\|check` |
| Tests | `cargo nextest run --workspace` |
| Quality Gates | `./scripts/quality-gates.sh` |

## Project Structure

- `.agents/skills/`: Executable task knowledge and canonical workflows.
- `crates/`: Workspace members (libraries and applications).
- `scripts/`: Development, quality, and release automation.
- `plans/adr/`: Architecture Decision Records.
- `AGENTS.md`: THIS FILE (Canonical Project Contract).

## Agent Skills (.agents/skills/)

Consult the relevant skill's `SKILL.md` for detailed procedures.

| Skill | Purpose |
|-------|---------|
| `build-rust` | Optimized build (uses `mold` if available) |
| `lint-rust` | Formatting and static analysis (zero clippy warnings) |
| `test-rust` | Comprehensive testing (nextest, proptest, fuzzing) |
| `release-rust` | Safe crate release process |
| `anti-ai-slop` | Auditing and fixing generic AI code patterns |
| `privacy-first` | PII and data leakage prevention (scripts/quality-gates.sh) |
| `crates-io-name-check` | Registry name availability verification |
| `codacy` | Codacy static analysis and PR triage |
| `metrics-reporter` | Recording agent task completion (per-task) |
| `dora-report` | Aggregated DORA and metrics reporting (monthly) |

## Session Bootstrap

The repository includes a `SessionStart` hook to auto-inject project context at the start of an agent session. This helps agents orient themselves quickly without manual discovery.

- **Hook:** `hooks/session-start.sh`
- **Config:** `docflow.json`
- **Integration:** Registered in `.claude/settings.json` for Claude.

## Coding Conventions

### Rust & Concurrency

- **Edition:** Rust 2024 (MSRV 1.88).
- **Safety:** `#![forbid(unsafe_code)]` at workspace and crate roots.
- **Errors:** `thiserror` for libraries, `anyhow` for binaries. No `unwrap()` in libs.
- **Async:** Use `tokio`. CLI apps: prefer `#[tokio::main(flavor = "current_thread")]`.
- **Async Safety:** Avoid blocking calls; use `spawn_blocking` when necessary.
- **Tracing:** Minimize CLI tracing metadata (thread IDs/names) unless high-concurrency.

### Security & Configuration

- **Hardening:** Enforce `#[serde(deny_unknown_fields)]` on config structs.
- **Safe Loading:** Use `file.take(limit)` and `is_file()` check before reading.
- **Validation:** Sanitize strings (`is_control()`) and enforce bounds on numeric fields.
- **Dependencies:** Pin core crates (`=1.0.x`). Audit with `cargo tree` and `deny.toml`.
- **Secrets:** Never hardcode; use environment variables or `.env`.

### Quality & Workflow

- **File Size:** Max 500 LOC per source file.
- **Docs:** All public items must have `///` doc comments.
- **TDD:** Add or update tests before implementing logic.
- **Search:** Always use `--exclude-dir=target` (and `.git`) in search commands.
- **Context:** Run `bash scripts/generate-llms-txt.sh` after significant arch changes.

## Mandatory Pre-Push Rules (ALL agents and bots)

> These rules are REQUIRED before every `git push`. CI is a verifier,
> not the first place failures should be caught. Violations cause red PRs
> and waste CI minutes.

### Formatting

1. After writing or modifying any `.rs` file, run `cargo fmt --all`.
2. Before staging a commit, verify with `cargo fmt --all -- --check`.
3. If the check fails, run `cargo fmt --all`, re-stage, and re-commit.
4. **Never push with a failing `cargo fmt --check`.**

### Quality Gates

5. Before every `git push`, run `bash scripts/quality-gates.sh`.
6. If any gate fails, fix the issue before pushing. Do not skip or bypass.

### Commit Messages

7. Use Conventional Commits: `<type>(<scope>): <summary>`.
8. **Wrap every line in the commit body at 100 characters or less** (`body-max-line-length`).
9. **Wrap every line in the commit footer at 100 characters or less** (`footer-max-line-length`).
10. If generating a long description, split it into short wrapped lines. Do not write
    one long sentence as a single body line.
11. Use `.gitmessage.txt` as a reference for the expected format.

### Example of a valid commit body

```
feat(mcp): add tool validation in McpServer::execute_tool

Prevent panics on non-finite float results by replacing unwrap() with
proper error handling. Return a structured error when serde_json
cannot represent the value (e.g. Infinity or NaN).

Fixes: #160
```

### Example of an INVALID commit body (violates body-max-line-length)

```
feat(mcp): add tool validation in McpServer::execute_tool

- Replaced `unwrap()` on `serde_json::Number::from_f64()` with proper error handling in `CalcTool::handle` to prevent panics on Infinity or NaN results.
```

## Change Workflow

1. **Discover:** Read code patterns, module structure, and `.agents/aggregated/ci-summary.md`.
2. **Plan:** Identify affected files and required test coverage.
3. **Test-First:** Add or update tests before logic implementation.
4. **Implement:** Write code adhering to conventions.
5. **Format:** Run `cargo fmt --all` and verify with `cargo fmt --all -- --check`.
6. **Quality Check:** Run `./scripts/quality-gates.sh`.
7. **Commit:** Use Conventional Commit format. Wrap body lines at 100 chars.
8. **Push:** Only push after all local checks pass.

## Agentic Metrics Reporting

After completing any task, write a JSON event file to `.agents/events/YYYY/MM/DD/`.
This event-based pattern is also used for CI benchmarks in `benchmarks/events/` to prevent
merge conflicts. See `.agents/skills/metrics-reporter/` for the schema and event writing
procedures. Set `human_interventions > 0` if a human corrected your code or provided rework
instructions.

## Release Failures (Priority 1)

If a `release-failure` issue is open:

1. Create a `hotfix/` branch and apply the minimal fix.
2. Open a PR with the `hotfix` label.
3. Close the issue with: `Recovered at: <TIMESTAMP>. FDRT: <HOURS>`.
