# Agent Coding Contract

> **2026 Best Practice Rust Template** - This is the single canonical instruction file for all AI agents.
> Tool-specific files (`CLAUDE.md`, `.cursorrules`, etc.) are thin adapters that point here.

## Quick Reference

| Task | Command |
|------|----------|
| Build | `cargo build --workspace` |
| Quality | `./scripts/quality-gates.sh` |
| Tests | `cargo nextest run --workspace` |
| Setup | `./scripts/bootstrap.sh` |
| Diagnostics | `./scripts/doctor.sh` |

## Project Structure

- `.agents/skills/`: Executable task knowledge and canonical workflows.
- `.agents/skills/harness/`: Harness engineering — sensor response protocol and self-correction.
- `crates/`: Workspace members (libraries and applications).
- `scripts/`: Development, quality, and release automation.
- `plans/adr/`: Architecture Decision Records.
- `.githooks/`: Pre-commit quality enforcement.
- `AGENTS.md`: THIS FILE (Canonical Project Contract).

## Agent Skills (.agents/skills/)

The skills index is **auto-generated** from skill frontmatter. Do not edit the table below manually.

**Regenerate after adding/modifying skills:**

```bash
bash scripts/generate-skills-md.sh
```

<!-- AUTO-GENERATED: see .agents/SKILLS.md for full table -->
Consult `.agents/SKILLS.md` for the complete skills index, or read individual skill docs at `.agents/skills/<name>/SKILL.md`.

## Multi-Agent Support

Skill symlinks are managed automatically. After cloning, run:

```bash
./scripts/bootstrap.sh    # One-command setup
./scripts/doctor.sh        # Environment diagnostics
```

CLI-specific directories read from `.agents/skills/` via symlinks:
- `.claude/skills/` → Claude Code
- `.qwen/skills/` → Qwen Code
- `.gemini/`, `.opencode/`, `.windsurf/` → Read directly

## Session Bootstrap

The repository includes a `SessionStart` hook to auto-inject project context at the start of an agent session. This helps agents orient themselves quickly without manual discovery.

- **Hook:** `hooks/session-start.sh`
- **Config:** `docflow.json`
- **Integration:** Registered in `.claude/settings.json` for Claude.

## Cross-Repo Context

Derived repositories should check `.agents/context/` for shared conventions and related repository links.

- **`.agents/context/external-repos.json`**: Links to related repos and their agent context URLs
- **`.agents/context/shared-conventions.md`**: Cross-repo coding conventions (commit format, branch naming, PR requirements)

**Merge precedence**: Local repo instructions > imported context > template defaults.

## Coding Conventions

### Rust & Concurrency
- **Edition:** Rust 2024 (MSRV 1.88). Edition 2024 requires ≥1.85; 1.88 is intentional for broader codespace/toolchain compatibility. Do not bump MSRV unless required by a dependency. https://doc.rust-lang.org/edition-guide/rust-2024/index.html
- **Versions:** `VERSION` / workspace `0.0.0` is the **adopter app** version (start here). Template meta-releases (e.g. v0.3.x) live only in `.template/CHANGELOG-TEMPLATE.md` — do not sync them into `VERSION`.
- **Safety:** `#![forbid(unsafe_code)]` at workspace and crate roots.
- **Errors:** `thiserror` for libraries, `anyhow` for binaries. No `unwrap()` in libs.
- **Async:** Use `tokio` when you need a runtime. CLI apps that use async: prefer `#[tokio::main(flavor = "current_thread")]`. Sync `main` is fine when no async is required (`sample-app`).
- **Async Safety:** Avoid blocking calls; use `spawn_blocking` when necessary.
- **Tracing:** Minimize CLI tracing metadata (thread IDs/names) unless high-concurrency.
- **Quality SSOT:** Prefer `./scripts/quality-gates.sh` before push. `cargo run -p xtask quality-gates` delegates to that script.

### Security & Configuration
- **Hardening:** Enforce `#[serde(deny_unknown_fields)]` on config structs.
- **Safe Loading:** Use `file.take(limit)` and `is_file()` check before reading.
- **Validation:** Sanitize strings (`is_control()`) and enforce bounds on numeric fields.
- **Dependencies:** Declare versions in `[workspace.dependencies]` with caret ranges (e.g. `"1"` ≡ `^1`). Library/template crates should ignore `Cargo.lock` (rely on `Cargo.toml` constraints); binary applications should commit it for reproducible builds. Audit with `cargo tree` and `deny.toml`. Prefer lockfile pins over exact `=` requirements in manifests. https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html
- **Secrets:** Never hardcode; use environment variables or `.env`.
- **Template Portability:** Never hardcode project name, repo URL, or author across source files. All project-specific values must derive from `Cargo.toml` at runtime or be rewriteable via `scripts/init-template.sh`. Avoid magic number thresholds — define named constants.

### Quality & Workflow
- **File Size:** Max 500 LOC per source file.
- **Docs:** All public items must have `///` doc comments.
- **TDD:** Add or update tests before implementing logic.
- **Search:** Always use `--exclude-dir=target` (and `.git`) in search commands.
- **Context:** Run `bash scripts/generate-llms-txt.sh` after significant arch changes.
- **Commits:** Strictly use lowercase for the subject line (e.g., `fix(scope): add ...` not `fix(scope): Add ...`). Sentence-case or start-case will fail CI.

## Change Workflow

1. **Discover:** Read code patterns, module structure, and `.agents/ci/ci-summary.md`.
2. **Plan:** Identify affected files and required test coverage.
3. **Test-First:** Add or update tests before logic implementation.
4. **Implement:** Write code adhering to conventions.
5. **Quality Check:** Run `./scripts/quality-gates.sh`.
6. **Commit:** Use conventional commit format.

## Agentic Metrics Reporting

After completing any task, write a JSON event file to `.agents/events/YYYY/MM/DD/`.
This event-based pattern is also used for CI benchmarks in `benchmarks/events/` to prevent merge conflicts.
See `.agents/skills/metrics-reporter/` for the schema and event writing procedures.
Set `human_interventions > 0` if a human corrected your code or provided rework instructions.

## Release Failures (Priority 1)

If a `release-failure` issue is open:
1. Create a `hotfix/` branch and apply the minimal fix.
2. Open a PR with the `hotfix` label.
3. Close the issue with: `Recovered at: <TIMESTAMP>. FDRT: <HOURS>`.
