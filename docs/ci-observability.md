# CI Observability (issue #289)

Every `xtask quality run` (which is what the CI `quality-gate` job invokes) emits
a structured telemetry artifact and a readable summary — no external SaaS required.

## Artifact

`.agents/ci/quality-run.json` (schema `schema/ci-telemetry.schema.json`, `schema_version: 1`):

- `tier` — which verification tier ran (e.g. `pull-request`, `protected-branch`).
- `plan_source` — where the check plan came from (`config/xtask.json`).
- `scope` — `affected-packages` (plus the crate list) when a `--changed-from` base was
  supplied, otherwise `all`; `fallback_used` is set when the base could not be resolved.
- `stages` — every configured check with `passed`/`failed`/`skipped`, wall-clock
  `duration_ms`, cache state, and an explicit `skipped_reason`.
- `toolchain` — `rustc`/`cargo`/`cargo-nextest` versions captured at run time.

The matching `.agents/ci/quality-summary.md` is the human-readable rendition and is also
appended to the GitHub Actions step summary. CI uploads `quality-run.json` as the
`ci-telemetry` artifact (retained per `config/ci/telemetry.toml`).

## Guarantees

- **Always emitted**: telemetry is written even when a stage fails (the gate reports the
  overall failure *after* emitting).
- **No secrets**: the schema contains no source content, tokens, env dumps, or credentials.
- **Configurable**: budgets and behaviour live in `config/ci/telemetry.toml`
  (`enabled`, `detail`, `retention_days`, `max_stage_duration_ms`), never in xtask logic.
- **Portable**: downstream repos can export `quality-run.json` to their own observability
  stack, or disable telemetry entirely.

## Budgets

Stages that exceed `budgets.max_stage_duration_ms` are flagged in the summary.
