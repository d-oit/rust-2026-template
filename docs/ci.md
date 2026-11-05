# CI Tier Model

This document describes the tiered CI strategy used by the rust-2026-template.
The goal is to keep fast-feedback checks on every PR while reserving expensive
jobs for scheduled runs or manual dispatch, reducing CI cost and PR cycle time.

## Tier overview

| Tier | Jobs | Trigger |
|------|------|---------|
| **1 — Fast required** | Quality Gate, lint metadata, version-check, validate-agents, Gitleaks | Every PR and push (path-scoped) |
| **2 — Conditional** | bench; path-selected MSRV and coverage inside the Quality Gate | Path-scoped: only when Rust code, benchmarks, or fuzz targets change |
| **3 — Scheduled / heavy** | mutation testing, fuzzing | Weekly schedule + `workflow_dispatch` (manual) |
| **4 — Release only** | dist, publish, post-release health check | Tag push (`v*.*.*`) |
| **5 — Template maintenance** | sync-labels, DORA report, DORA FDRT, patch-release-on-label, hotfix tracking | Schedule, label, or issue events |

## Tier 1 — Fast required

These jobs run on **every PR** that touches code, workflows, agent docs, or
Markdown. `Quality Gate (pull-request)` owns the canonical xtask checks, and
`CI Success` is the stable aggregate merge-gate context.

- **Quality Gate** — `xtask` runs format, Clippy, build, tests, workspace
  doctests, root dependency audit/policy, shell, Markdown, YAML, workflow,
  privacy, and secret checks. CI sets `XTASK_TEST_COVERAGE=1`, so the single
  `Test` check runs `cargo llvm-cov nextest`; local quality gates use plain
  `cargo nextest` unless that CI-only variable is set. LCOV from that run is
  uploaded to Codecov.
- **lint** — template placeholder and agent-metrics metadata checks only.
- **gitleaks** — secret scanning on full git history in `security-scan.yml`.
- **version-check** — `VERSION` file matches `Cargo.toml`; generated context
  and skill indexes are current.
- **validate-agents** — agent entrypoint consistency check.

### Check ownership

Each event has one blocking owner for each quality or dependency check:

| Event | Blocking owner | Check set |
|---|---|---|
| Pull request | `ci.yml` → `quality-gate` → `xtask` `pull-request` tier | Format, Clippy, build, tests and coverage, workspace doctests, audit, deny, shell/Markdown/YAML lint, workflow validation, privacy and secret checks |
| Protected-branch push | `ci.yml` → `quality-gate` → `xtask` `protected-branch` tier | Same checks plus machete |
| Scheduled/manual security scan | `security-scan.yml` → `cargo-audit` / `cargo-deny`; Gitleaks remains in this workflow | Root workspace advisories, dependency policy and secrets |
| Scheduled/manual fuzz scan | `fuzz.yml` → `fuzz-dependencies` | Separate fuzz workspace lockfile, advisory and dependency policy checks; fuzzing itself runs only outside pull requests |
| Release | Release workflow's configured xtask tier | Release build, tests, doctests, audit and deny |

Standalone CI jobs do not rerun fmt, Clippy, tests, audit, or deny. Scheduled
security jobs retain independent audit/deny runs for ad-hoc and scheduled
scanning; on pull requests and pushes those blocking root-graph checks belong
to xtask. The heuristic Roast Scorer has been removed; deterministic quality
checks remain the only quality gates.

### Lockfile Security Scanning Policy

Root-graph audit and deny checks run through xtask for pull requests and
protected-branch pushes. `.github/workflows/security-scan.yml` retains
scheduled/manual scans and adapts to template initialization profiles:

- **Application profiles (committed `Cargo.lock`):** Audits and validates the exact committed lockfile without regenerating or mutating it. Enforces lockfile preservation post-scan (`git diff --exit-code Cargo.lock`).
- **Library profiles (untracked `Cargo.lock`):** Dynamically resolves a temporary lockfile in an isolated step to scan the latest compatible dependency graph against advisories and policies.

The fuzz workspace is excluded from root Cargo workspace checks. The Fuzz
workflow runs `scripts/check-fuzz-deps.sh` when fuzz inputs, root dependency
policy/manifests, or the checker change, and on scheduled/manual runs. The
checker validates the separate fuzz dependency graph with locked metadata,
audit, and deny checks; it does not add a fuzz-only gate to the root xtask
tiers.

### Path scoping

Jobs are skipped when only unrelated files change. A Markdown-only PR still
runs the quality gate for Markdown lint, while its changed-path plan skips
Rust build and test checks. The `changes` job at the top of `ci.yml` uses
`dorny/paths-filter` to detect:

- `code` — Rust source, Cargo manifests, toolchain, tests, examples, and
  Markdown files
- `heavy` — subset of `code` excluding tests (benchmarks, fuzz targets)
- `agents` — AGENTS.md, agent-docs, scripts
- `workflows` — `.github/workflows/**`
- Markdown changes — run the xtask-owned Markdown lint check without rerunning
  the Rust compilation and test checks.

### Expected-Skip Contract and Merge Gate Aggregation

The `ci-success` job serves as the required merge-gate check context (`CI Success`).
To prevent fail-open regressions where broken path filters or missing dependencies
allow required jobs to be skipped silently, `ci-success` enforces an explicit
fail-closed evaluation contract via `scripts/ci-required-evaluate.sh`:

- **Success accepted**: Any job reporting `success` is accepted.
- **Skipped accepted only when justified**: A job result of `skipped` is accepted
  **only** during a `pull_request` event when the path-filter contract proves the
  job was not required:
  - `quality-gate`: skipped only if neither `code` nor `workflows` changed.
  - `bench`: skipped only if neither `heavy` nor `workflows` changed.
  - `validate-agents`: skipped only if neither `agents` nor `workflows` changed.
  - `lint`, `version-check`: skipped only if none of `code`, `agents`, or `workflows` changed.
- **Fail-closed on unexpected results**: If a job was required by the path-filter contract
  and was skipped, or if any job reports `failure`, `cancelled`, `timed_out`, `action_required`,
  missing results, or if the `changes` detection job itself did not succeed, the aggregate
  fails immediately with actionable diagnostic output and remediation hints.
- **Push and manual dispatch**: On non-PR events (`push`, `workflow_dispatch`), all jobs
  must run; no skipped jobs are permitted.

## Tier 2 — Conditional

These jobs run only when the affected paths match:

- **bench** — compiles and runs benchmarks (`heavy` or `workflows` paths)
- **MSRV** — runs from the xtask quality tier when Rust sources, manifests, or the toolchain change

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

The default CI workflow uses **least-privilege** permissions:

- **Default (workflow level):** `contents: read`
- CI summaries and quality telemetry are uploaded as artifacts and written to
  `$GITHUB_STEP_SUMMARY`; CI does not push commits or persist generated files
  to product branches.
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

## Configuration-driven verification tiers (`config/xtask.json`)

To decouple lifecycle policies from GitHub Actions YAML, quality checks are managed by `config/xtask.json`. Downstream adopters and template profiles configure which checks run for each lifecycle trigger (`pull-request`, `protected-branch`, `scheduled`, `release`) without hardcoding values in workflow files.

The configuration file structure is validated against `schema/xtask-config.schema.json`:

```json
{
  "env_var_name": "XTASK_TIER",
  "default_tier": "protected-branch",
  "tiers": {
    "pull-request": {
      "checks": [
        "LocLimits",
        "Fmt",
        "Clippy",
        "Build",
        "Test",
        "DocTest",
        "Audit",
        "Deny",
        "ShellCheck",
        "MarkdownLint",
        "YamlLint",
        "PrivacyCheck",
        "SecretScan"
      ]
    },
    "protected-branch": {
      "checks": [
        "LocLimits",
        "Fmt",
        "Clippy",
        "Build",
        "Test",
        "DocTest",
        "Audit",
        "Deny",
        "Machete",
        "Msrv",
        "ShellCheck",
        "MarkdownLint",
        "YamlLint",
        "PrivacyCheck",
        "SecretScan",
        "WorkflowValidation",
        "ActionLint"
      ],
      "required_checks": [
        "Audit",
        "Deny",
        "ShellCheck",
        "MarkdownLint",
        "YamlLint",
        "WorkflowValidation",
        "ActionLint",
        "SecretScan"
      ]
    }
  },
  "lint_thresholds": {
    "max_lines_per_file": 500,
    "clippy_warnings_as_errors": true
  }
}
```

Run quality gates for a specific tier locally or in CI:

```bash
cargo run -p xtask -- quality run --tier pull-request
```

## Extending for downstream repos

Generated repos inherit this CI configuration. To add stricter checks:

1. Add new checks or tier definitions in `config/xtask.json`
2. Add new jobs to the appropriate tier section in `ci.yml` if external workflow steps are needed
3. Update the `ci-success` job's `needs` array if adding required checks
4. Document new jobs in this file
