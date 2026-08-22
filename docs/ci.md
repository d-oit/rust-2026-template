# CI Tier Model

This document describes the tiered CI strategy used by the rust-2026-template.
The goal is to keep fast-feedback checks on every PR while reserving expensive
jobs for scheduled runs or manual dispatch, reducing CI cost and PR cycle time.

## Tier overview

| Tier | Jobs | Trigger |
|------|------|---------|
| **1 — Fast required** | fmt, clippy, test, audit, deny, lint, shellcheck, gitleaks, version-check, validate-agents | Every PR and push (path-scoped) |
| **2 — Conditional** | bench, msrv, coverage (inside test) | Path-scoped: only when Rust code, benchmarks, or fuzz targets change |
| **3 — Scheduled / heavy** | mutation testing, fuzzing | Weekly schedule + `workflow_dispatch` (manual) |
| **4 — Release only** | dist, publish, post-release health check | Tag push (`v*.*.*`) |
| **5 — Template maintenance** | sync-labels, DORA report, DORA FDRT, patch-release-on-label, hotfix tracking | Schedule, label, or issue events |

## Tier 1 — Fast required

These jobs run on **every PR** that touches code, workflows, or agent docs.
They complete in under 2 minutes on the default `ubuntu-latest` runner.

- **fmt** — `cargo fmt --all -- --check`
- **clippy** — `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- **test** — `cargo nextest run` + `cargo test --doc` (with coverage via `cargo-llvm-cov`)
- **security** — `cargo audit`
- **deny** — `cargo deny check`
- **lint** — YAML lint, Markdown lint, commitlint, agent metrics schema validation
- **shellcheck** — static analysis of all shell scripts in `scripts/`
- **gitleaks** — secret scanning on full git history
- **version-check** — `VERSION` file matches `Cargo.toml`; `llms.txt` is current
- **validate-agents** — agent entrypoint consistency check

### Path scoping

Jobs are skipped when only unrelated files change (e.g., docs-only PRs skip
Rust build jobs). The `changes` job at the top of `ci.yml` uses
`dorny/paths-filter` to detect:

- `code` — Rust source, Cargo manifests, toolchain, tests, examples
- `heavy` — subset of `code` excluding tests (benchmarks, fuzz targets)
- `agents` — AGENTS.md, agent-docs, scripts
- `workflows` — `.github/workflows/**`

## Tier 2 — Conditional

These jobs run only when the affected paths match:

- **bench** — compiles and runs benchmarks (`heavy` or `workflows` paths)
- **msrv** — checks minimum supported Rust version (`heavy` or `workflows` paths)

> **Note:** Docs sync checking is handled by a separate workflow
> (`.github/workflows/docs-check.yml`), not by `ci.yml`.

## Tier 3 — Scheduled / heavy

These jobs are **too expensive for every PR**. They run on:

- **Weekly schedule** (cron)
- **Manual dispatch** (`workflow_dispatch`)
- **Push to main/develop** (so merged code is still validated)

| Job | Schedule | Duration |
|-----|----------|----------|
| Mutation testing (`cargo-mutants`) | Monday 03:00 UTC | ~30 min |
| Fuzz testing (`cargo-fuzz`) | Sunday 02:00 UTC | ~5 min |

> **Why no PR trigger?** Mutation testing can take 30+ minutes and fuzzing
> requires nightly Rust. Both are better validated post-merge or on-demand.
> Use `workflow_dispatch` to run them manually when reviewing a risky change.

## Tier 4 — Release only

Triggered by pushing a version tag (`v*.*.*`):

1. Full CI (reusable workflow call to `ci.yml`)
2. `cargo dist build` + GitHub Release creation
3. `cargo publish` to crates.io
4. Post-release health check (smoke test + DORA FDRT tracking)

## Tier 5 — Template maintenance

These jobs are specific to the template repository and would self-disable in
generated repos:

- **sync-labels** — monthly label discovery from repo activity
- **DORA report** — monthly automated DORA metrics report
- **DORA FDRT** — records failed deployment recovery time on issue close
- **patch-release-on-label** — version bump triggered by `release:patch` label
- **hotfix tracking** — records hotfix/rework events for DORA CFR metric

## Permissions model

All workflows use **least-privilege** permissions:

- **Default (workflow level):** `contents: read`
- **Jobs needing write access** (e.g., `ci-success` persisting CI data):
  declare `permissions: contents: write` at the job level
- **Release workflow:** elevated permissions scoped to `contents: write`,
  `packages: write`, `issues: write`, `id-token: write`

## Action governance

All third-party actions are **SHA-pinned** (not tag-pinned) to prevent
supply-chain attacks. Each pinned reference includes a version comment for
readability:

```yaml
uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd  # v6
```

Dependabot is configured to propose SHA-pinned updates via PRs.

## Caching strategy

- **Cargo registry + git:** via `Swatinem/rust-cache@v2` (keyed on `Cargo.lock`
  and toolchain)
- **sccache:** via `mozilla-actions/sccache-action` for compiled artifact reuse
- **Tool installation:** `taiki-e/install-action` caches installed binaries

Cache invalidation is automatic on lockfile or toolchain changes.

## Extending for downstream repos

Generated repos inherit this CI configuration. To add stricter checks:

1. Add new jobs to the appropriate tier section in `ci.yml`
2. Update the `ci-success` job's `needs` array if adding required checks
3. Document new jobs in this file
