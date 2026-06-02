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
├── .agents/events/      # Per-task event files (conflict-free, append-only)
├── .agents/aggregated/  # CI-generated metrics summaries (do not hand-edit)
├── .agents/context/     # Shared workflow state for multi-agent coordination
├── .cargo/config.toml   # Cargo linker + profile config
├── .config/nextest.toml # nextest profiles
├── .github/workflows/   # CI/CD GitHub Actions
├── scripts/             # Development and quality scripts
├── plans/adr/           # Architecture Decision Records
├── AGENTS.md            # THIS FILE (Canonical Guidance)
├── CLAUDE.md            # Claude-specific reference (@AGENTS.md)
├── GEMINI.md            # Gemini-specific reference (@AGENTS.md)
├── QWEN.md              # Qwen-specific reference (@AGENTS.md)
├── llms.txt             # LLM context file (machine-readable project overview)
├── llms-full.txt        # Full LLM context (auto-generated)
├── VERSION              # Plain-text version file (single source of truth)
└── Cargo.toml           # Workspace manifest
```

## Agent Skills (.agents/skills/)

Specialized workflows are defined as "skills". Always consult the relevant skill's `SKILL.md` for detailed procedures.
For multi-agent orchestration, skill chaining, and handoff protocol, see **[`.agents/ORCHESTRATION.md`](.agents/ORCHESTRATION.md)**.

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
| `codacy` | Codacy static analysis and PR triage workflows (see `.codacy/` for tool configs) |
| `dora-report` | Automated DORA and agentic metrics reporting (run monthly) |

## Responding to Release Failures

If you see an open issue with label `release-failure`:
1. This is the **highest priority** task. Stop other work.
2. Read the workflow logs linked in the issue body.
3. Create a `hotfix/fix-description` branch.
4. Apply the minimal fix required to restore functionality.
5. Open a PR with label `hotfix` (see issue #99 for CFR tracking).
6. After the PR is merged and re-release succeeds, close the `release-failure` issue with comment: `Recovered at: YYYY-MM-DDTHH:MM:SSZ. FDRT: X.X hours`.

## Change Workflow

1. **Discover:** Read existing code patterns, module structure, and `ci-summary.md` to ensure a healthy baseline.
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
- [ ] Check `ci-summary.md` for current CI status

## Code Conventions

- **File Size:** Max 500 LOC per source file; refactor into submodules if exceeded.
- **Lints:** Zero `clippy` warnings allowed. Do not suppress warnings without extreme justification.
- **Concurrency:** Prefer `tokio` for async logic. Avoid blocking calls in async contexts.
- **Error Handling:** Use `thiserror` for libraries and `anyhow` for applications/binaries.
- **Safety:** `unsafe_code = "forbid"` is enforced at the manifest level. No `unwrap()` in library code.
- **Documentation:** All public items must have `///` doc comments.
- **Testing:** Use `proptest` for pure functions and `tokio::test` for async logic.
- **Lint Phases:** The lint setup uses a phased approach:
  - **Phase A (Active):** High-signal lints enabled immediately: `float_cmp`, `significant_drop_tightening`, `cast_precision_loss`, `cast_possible_truncation`, `redundant_clone`, `map_unwrap_or`, `unnecessary_map_or`, `missing_const_for_fn`. Fix all warnings from these lints before merging.
  - **Phase B:** Enable when codebase is stable enough to fix all violations systematically.
  - **Phase C (Pre-release):** Enable `missing_errors_doc` and `must_use_candidate` before v1.0 public release or crates.io publish. Flip from `allow` to `warn` in `[lints.clippy]`.
  - When adding new lints, always document the phase and rationale inline in `Cargo.toml`.

## Core Invariants

- **Performance:** Use `mold` linker and optimized dev profiles (see `.cargo/config.toml`).
- **Security:** Never hardcode secrets; use environment variables or a `.env` file.
- **Privacy:** Adhere to the `privacy-first` skill to avoid leaking PII.
- **Context Files:** Regenerate `llms.txt` and `llms-full.txt` after significant architectural changes via `bash scripts/generate-llms-txt.sh`.
- **Lint Phases:** The lint setup uses a phased approach. Promoted pedantic/nursery lints are Phase A (enabled now). Phase C lints (`missing_errors_doc`, `must_use_candidate`) should be flipped to `warn` before v1.0 release.

## Agentic Metrics Reporting

After completing any task, write a **per-event JSON file** to `.agents/events/`. This is conflict-free and supports multiple agents, multiple skills, and multiple tasks running in parallel.

> ⚠️ **Deprecated:** Do not append directly to `.agents/metrics.jsonl`. That pattern causes Git merge conflicts when multiple agents work in parallel. Use event files instead.

### Write an event file (bash)

```bash
# Required: AGENT_NAME, TASK_ID, SKILL_NAME
# Optional: WORKFLOW_ID, PARENT_TASK_ID, AGENT_TYPE, SKILL_VERSION, PR_NUMBER, NOTES

EVENT_DIR=".agents/events/$(date -u +%Y/%m/%d)"
mkdir -p "${EVENT_DIR}"
EVENT_FILE="${EVENT_DIR}/$(date -u +%Y-%m-%dT%H-%M-%SZ)-${AGENT_NAME:-unknown}-${TASK_ID:-$(date -u +%s)}.json"

cat > "${EVENT_FILE}" <<EOF
{
  "event_id": "$(date -u +%s)-${AGENT_NAME:-unknown}",
  "task_id": "${TASK_ID:-$(date -u +%s)}",
  "parent_task_id": ${PARENT_TASK_ID:-null},
  "workflow_id": ${WORKFLOW_ID:-null},
  "agent_id": "${AGENT_NAME:-unknown}",
  "agent_type": "${AGENT_TYPE:-unknown}",
  "skill": "${SKILL_NAME:-unknown}",
  "skill_version": "${SKILL_VERSION:-0.0.0}",
  "status": "${STATUS:-success}",
  "started_at": "${STARTED_AT:-$(date -u +%Y-%m-%dT%H:%M:%SZ)}",
  "finished_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "success": ${SUCCESS:-true},
  "human_interventions": ${HUMAN_INTERVENTIONS:-0},
  "git_branch": "$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)",
  "git_sha": "$(git rev-parse --short HEAD 2>/dev/null || echo unknown)",
  "pr_number": ${PR_NUMBER:-null},
  "artifacts": [],
  "notes": "${NOTES:-}"
}
EOF
echo "Event written: ${EVENT_FILE}"
```

### When to set `human_interventions > 0`

- A human had to correct, rewrite, or revert agent-produced code
- The PR required a fixup commit after agent submission
- A review comment explicitly called out an agent error

### Aggregating metrics (CI)

Run `bash scripts/aggregate-metrics.sh` in CI after merging to rebuild
`.agents/aggregated/metrics.jsonl` and `.agents/aggregated/daily-summary.json` from all event files.
These files are **generated output** — never hand-edit them.

## Cross-References

| Topic | Document |
|-------|----------|
| Multi-Agent Orchestration | `.agents/ORCHESTRATION.md` |
| Workflow State | `.agents/context/workflow-state.json` |
| DORA Metrics | `docs/dora-metrics.md` |
| Detailed Commands | `agents-docs/commands.md` |
| Code Structure | `agents-docs/structure.md` |
| Coding Conventions | `agents-docs/conventions.md` |
| Workflow Details | `agents-docs/workflow.md` |
| DORA Metrics | `agents-docs/dora-metrics.md` |
| Architecture | `plans/adr/` |
| Skills Directory | `.agents/skills/` |
