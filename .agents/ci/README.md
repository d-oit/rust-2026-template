# `.agents/ci` — CI Status & Telemetry Artifacts

This directory holds the artifacts the quality gate produces and CI publishes.

## Files

| Path | Kind | Content |
|---|---|---|
| `ci-status.json` | Status | Consolidated result of the last quality gate run (`QualityReport`). Committed by CI on `main`. |
| `ci-summary.md` | Status | Human-readable Markdown rendition of `ci-status.json`. |
| `quality-run.json` | Telemetry | Schema-versioned telemetry for the last run: tier, plan source, scope, per-stage timing/skip reasons, toolchain versions (see `schema/ci-telemetry.schema.json`). Uploaded as a GH Actions artifact, not committed. |
| `quality-summary.md` | Telemetry | Readable summary of `quality-run.json`, also appended to the job summary. |

## Lifecycle

- Both `ci-status.json` and `ci-summary.md` are **committed to `main`** by the CI
  `ci-success` job (so the default branch always shows the last gate result).
- `quality-run.json` / `quality-summary.md` are **volatile artifacts**: generated on every
  run, uploaded by the `quality-gate` job, and never committed (see `.gitignore`).

## Disabling telemetry

Set `enabled = false` in `config/ci/telemetry.toml`. The gate still writes
`ci-status.json`/`ci-summary.md`, but stops emitting the telemetry pair.
