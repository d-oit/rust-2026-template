---
name: harness
description: >
  Harness engineering guide — maps all sensors and feedforward guides,
  teaches self-correction protocol when a CI sensor fires.
  Use when a sensor fires, before making code changes, or when setting up
  agent context for a new task.
  Triggers: "harness", "sensor fire", "CI failure", "self-correction".

license: MIT
metadata:
  category: rust
  author: d-oit
  version: "1.0"
  tags: harness sensors ci quality self-correction
---

# Skill: harness

## What Is the Harness

Agent = Model + Harness. The harness is the system of feedforward guides (what to do before coding) and feedback sensors (what catches violations after coding). It has two axes:

- **Feedforward (guides):** Context, constraints, conventions that prevent errors before they happen.
- **Feedback (sensors):** Automated checks that fire after code changes, providing structured error output.

The harness has two modes:
- **Computational:** Deterministic checks (fmt, clippy, deny) — always trust the output.
- **Inferential:** LLM-based guidance (skill docs, agent context) — direction, not commands.

## Tool Ownership & Quality Verification

- **Canonical Project Quality Command:** `./scripts/quality-gates.sh` (or `cargo run -p xtask -- quality run`). `xtask` is the single source of truth for project quality tiers (`pull-request`, `protected-branch`, `scheduled`, `release`) and quality telemetry (`.agents/ci/quality-run.json`).
- **Optional Agent Harness Tooling:** `do-harness` (`scripts/setup-harness.sh`) provides agent diagnostics (`do-harness doctor`) and PR readiness checks (`do-harness pr review`, `do-harness pr ready`). `do-harness.toml` delegates quality checks to `./scripts/quality-gates.sh` to prevent running redundant duplicate Cargo passes.
- **Non-Interference Rule:** Never assume a green status from `do-harness` means `xtask` checks ran, or vice versa. If `do-harness` is absent or PR context is unavailable, fall back directly to `./scripts/quality-gates.sh` and `./scripts/doctor.sh`.
- **Sensor Applicability Rule:** The `quality-gates` sensor's `when-changed` globs in `do-harness.toml` decide whether `do-harness verify --changed` runs it. Manifest edits need **both** `"Cargo.toml"` (workspace root manifest) and `"**/Cargo.toml"` (workspace member manifests, inside and outside `crates/`). Rust sources and lockfiles match on file shape (`"**/*.rs"`, `"**/Cargo.lock"`) so a file in an unknown directory still selects the sensor. `"Cargo.toml"` alone skips member manifests, and bare wildcards like `"**"` fire on Markdown-only edits. Keep the same rule in `config/xtask.json` `when_changed`.

## Sensor Response Protocol

When a computational sensor fires:

1. **Read the full error message** — it includes a fix hint.
2. **Classify the error:** fmt / lint / test / arch / security.
3. **Check historical regression signatures:** Consult `.agents/ci/regression-matrix.json` to see if the failure matches a known historical pattern (e.g. `MD001` heading jumps, `cargo-deny` policy violations, or `gitleaks` findings).
4. **Apply the minimal fix** — do not refactor unrelated code.
5. **Re-run the specific sensor** — `cargo fmt`, `cargo clippy`, etc.
6. **Only commit when the sensor is green.**
7. **Write a metrics event** — a per-event JSON file in `.agents/events/YYYY/MM/DD/` per the `metrics-reporter` skill (canonical Event File Format in `.agents/ORCHESTRATION.md`; do not append to the legacy `.agents/metrics.jsonl`).

## Sensor Quick Reference

| Sensor | Command | Config | Stage |
|--------|---------|--------|-------|
| fmt | `cargo fmt --all -- --check` | `.pre-commit-config.yaml` | pre-commit (fast) |
| clippy | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `.clippy.toml`, `.pre-commit-config.yaml` | pre-push only (`harness-check.sh` hook) + CI |
| deny | `cargo deny check` | `deny.toml` | pre-push only (`harness-check.sh` hook) + CI |
| nextest | `cargo nextest run --workspace` | `.config/nextest.toml` | CI |
| mutants | `cargo mutants --workspace --test-tool nextest` | `[workspace.metadata.cargo-mutants]` in `Cargo.toml` | CI: weekly Mon 03:00 UTC + push to `main`/`develop` (survivors are `continue-on-error`) |
| arch_fitness | `cargo test --test arch_fitness --package workspace-tests` | `crates/workspace-tests/tests/arch_fitness.rs` | local only (`harness-check.sh arch`; no workflow runs it) |
| insta snapshots | `make insta-test` / `cargo insta review` | `crates/workspace-tests/tests/behaviour_harness.rs` | CI |
| gitleaks | `gitleaks detect` | `.gitleaks.toml` | CI (`security-scan.yml`) |

## Snapshot Contract

Treat snapshot changes as behavior changes. Inspect them with `cargo insta review`; accept only intentional, deterministic output changes. Never update snapshots solely to make a failing test pass. CI runs the normal test suite without snapshot updates enabled, so pending snapshots fail rather than being accepted automatically.

## Steering Loop

When any sensor fires **repeatedly** (3 or more matches in one review — `distill-strikes.sh`'s default and recommended `--threshold 3`):

1. Run `./scripts/distill-strikes.sh --threshold 3` to cluster repeated failure signatures from `.agents/ci/regression-matrix.json` and recent quality run history (`.agents/ci/quality-run.json`).
2. If the fail-fast threshold is reached and no guide exists, `distill-strikes.sh` drafts a starter skill skeleton (`.agents/skills/drafts/<subsystem>/SKILL.md`) with verbatim HARNESS VIOLATION hints and a red fixture sample. If an existing skill exists, `distill-strikes.sh` refuses to overwrite and points to it instead.
3. Review and refine the draft skill with `skill-creator` principles. Draft promotion requires a green sensor verification run.
4. Update the corresponding **feedforward guide** to prevent recurrence and document the update in `CHANGELOG.md`.

The steering loop closes the harness: sensors fire → `distill-strikes.sh` drafts starter skills → humans and agents update guides → sensors fire less.

## Rationalizations

| Rationalization | Reality |
|---|---|
| "The sensor is just being pedantic, I'll bypass it." | Computational sensors exist to prevent regressions. Never suppress without approval. |
| "I'll fix this CI failure after committing." | Committing broken code breaks peers and pipelines. Always verify sensors pass before commit. |
| "I don't need to check the regression matrix." | Checking `.agents/ci/regression-matrix.json` saves time by identifying known failure patterns. |

## Red Flags

- [ ] Committing code while computational sensors are red
- [ ] Suppressing sensor errors rather than fixing root causes
- [ ] Ignoring repeated sensor strikes instead of running `distill-strikes.sh`

## References

- Full harness map: [`HARNESS.md`](../../../HARNESS.md) at repo root.
- Historical CI Regression Matrix: [`.agents/ci/regression-matrix.json`](../../ci/regression-matrix.json)
- Matrix test suite: [`tests/ci_regression_matrix_test.sh`](../../../tests/ci_regression_matrix_test.sh)
- Agent-optimised error output: `scripts/harness-check.sh` runs `fmt`, `clippy`, `deny`, `test`, `arch`, or `all` (`make harness`) and emits structured error output with `HARNESS VIOLATION` / fix-hint prefixes. Its `test` sensor runs `cargo nextest run --workspace`. Its `arch` sensor is wired into neither `harness-check.sh all` nor CI — run `bash scripts/harness-check.sh arch` explicitly after changing layering.
