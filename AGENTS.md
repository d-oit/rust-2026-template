# Agent Coding Contract

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
