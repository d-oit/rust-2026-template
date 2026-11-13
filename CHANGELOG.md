# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Added opt-in crates.io Trusted Publishing. `config/trusted-publishing.json` holds the explicit package allowlist and OIDC identity; `scripts/trustpub.sh` manages `plan`, `list`, `configure`, `verify`, `remove`, `bootstrap`, and enforcement; `release.yml` gains a validation-only `publish_plan` job and an OIDC `publish_crates` job that stores no long-lived registry secret. Offline contract and CLI tests run in CI and the release quality tier, and `docs/src/releasing/crates-io-trusted-publishing.md` documents the full lifecycle.
- Added a fail-closed generated-maintenance fast path for report-only pull requests. `config/maintenance-pr.json` centralizes a literal DORA artifact allowlist, the trusted automation identities, and the status-check identities; `scripts/classify-maintenance-pr.py` grants the fast path only when the PR targets `main`, comes from a recognized first-party automation actor and branch, every changed file matches the allowlist, and the complete changed-file list was retrieved. PR titles, bodies, and labels are never inputs, and globs or `..` segments in the allowlist are rejected. `scripts/validate-dora-artifacts.py` replaces the full tier for such PRs and cross-checks the report, manifest, and history artifacts so drifting one artifact fails the required check. The required context stays `Quality Gate (pull-request)`.
- Added `PR Merge Policy Gate` (`pr-merge-policy.yml`, `scripts/pr-merge-policy.py`), a repository-owned required check that replaces the unconditional Codacy context with a conditional policy: allowlisted maintenance PRs need the maintenance validators instead of Codacy, while every other PR needs a `success` check run named `Codacy Static Code Analysis` from Codacy app ID `56611` on the exact current head SHA. It paginates the full changed-file list, fails closed on truncation, API errors, wrong app IDs, pending or stale runs, and timeouts, and never trusts user-controlled metadata. `docs/ci/required-checks.md` documents the policy, the ruleset migration sequence, and the rollback procedure.
- Added offline contract suites: `tests/maintenance_classification_test.sh`, `tests/pr_merge_policy_test.sh`, `tests/workflow_maintenance_contract_test.sh`, and `tests/quality_plan_tools_test.sh`. They exercise the real CLIs against fixtures, require no credentials or network access, and run inside the required quality gate.
- Added an opt-in Rust-to-WASM guide (`docs/patterns/wasm.md`) covering target choice (`wasm32-unknown-unknown` vs a WASI target), pinned `wasm-bindgen-cli` tooling, `wasm-bindgen-test` runner selection, a non-zero-test guard against vacuous WASM suites, and release-only `wasm-opt` size budgets. It adds no toolchain target, dependency, or CI job to the default template.
- Added `scripts/check-fuzz-deps.sh`, which validates the excluded fuzz workspace with `cargo metadata --locked`, `cargo audit`, and `cargo deny` and fails with remediation guidance instead of silently regenerating a stale lockfile. `fuzz/Cargo.lock` is now tracked, and `fuzz.yml` runs the check when fuzz inputs or dependency policy change and on its scheduled run.
- Added `tests/ci_quality_ownership_test.sh`, a CI contract test that fails if a blocking fmt/Clippy/test/MSRV/audit/deny command is reintroduced in a standalone job, if the default workflow regains repository write permission or a commit/push step, or if the removed `CiStatusArtifact`/`RoastScorer` checks return.
- Added profile-pruning integration coverage proving `minimal`/`library` retain only the declared core Rust skills and that `.agents/SKILLS.md` and agent instructions are regenerated without references to removed skills.
- Added a doctest regression fixture with a restricted `default-members` list, proving the doctest gate executes a non-default workspace member and fails when that member's doctest breaks.
- Added an `actionlint` quality check (`GitHub Actions Semantic Check`) that validates GitHub Actions workflows semantically: undefined expressions, `needs` references to non-existent jobs, invalid contexts, and cron syntax. It runs on the `pull-request`, `protected-branch`, and `release` tiers, is required on the protected tiers, and installs `actionlint` v1.7.12 in CI. Only actionlint's own checks run — its bundled shellcheck and pyflakes passes are disabled because the repository lints shell separately and several workflow `run:` blocks rely on intentional word splitting.
- Added a `yamllint` quality check (`YAML Lint`) that mirrors the `yamllint .` run the CI lint job already performed, so YAML style violations are caught by `./scripts/quality-gates.sh` instead of only in CI. It is required on the `protected-branch` and `release` tiers.

### Changed

- `quality-gate` now classifies the PR before installing any toolchain and installs only the tool groups required by the resolved check plan (`scripts/quality-plan-tools.py`), so a Markdown-only PR no longer pays for Rust, security, shell, YAML, or actionlint setup. A selected required check whose tool is missing still fails closed inside xtask. Gitleaks, secretlint, CodeQL, and any workflow change continue to disqualify a PR from the fast path.
- Rewrote the weekly DORA report workflow: it checks out `main` explicitly, reuses the fixed `automation/dora-report` branch and its single PR, refreshes the branch only when it is behind, fails visibly on a conflicting update, and then enables `gh pr merge --auto --squash`. The commit message no longer contains `[skip ci]`, because a skipped required workflow would remain pending and block merge. Auto-merge never bypasses required checks or rules; no admin merge and no self-approval is used.
- `scripts/ci_required_evaluate.py` now understands the maintenance classification and accepts the fast path only when the file-set classifier granted it. A fast path claimed for a mixed code PR, for a non-PR event, or from an untrusted source is rejected. `scripts/validate-workflows.sh` now verifies static job names for every required context, not just `quality-gate`.
- Made `minimal` the common application profile and reduced `cli` to a documented compatibility alias for it, preserving the historical `protected-branch` CI tier, committed lockfile, and `generate-cli-manpage` checklist item. `sample-app` is documented as the binary/CLI starter.
- Trimmed generated project skill sets to the adopter's workflow: `minimal` and `library` keep only core Rust skills, `service` and `performance-sensitive` add advanced development skills, and `ai-agent`/`workspace` retain the broader set. Initialization regenerates `.agents/SKILLS.md` and prunes references to removed skills from `AGENTS.md`.
- Consolidated each blocking quality check under a single documented owner: `Quality Gate (pull-request)` runs the canonical xtask tier for format, Clippy, build, tests, workspace doctests, audit, deny, and lint checks; standalone `fmt`, `clippy`, `test`, `msrv`, and `shellcheck` jobs were removed. Coverage now comes from that single test execution via `XTASK_TEST_COVERAGE=1`.
- Promoted `pedantic` and `nursery` clippy lint groups from `allow` to `warn` at workspace level. Individual crates use local `[lints.clippy]` overrides where needed (Cargo 1.88 limitation).
- Expanded `tokio` features in `checkpoint-template` to include `rt-multi-thread`, `macros`, and `sync` (needed for `#[tokio::test]`).
- Bumped the pinned `do-harness` release in `scripts/setup-harness.sh` from `v0.3.4` to `v0.4.2`, a published release with a `checksums.txt` asset verified by the upstream installer. The generic `language = "generic"` delegation to `scripts/quality-gates.sh` is unchanged, so no duplicate Cargo pipeline is introduced.

### Deprecated

- (none)

### Removed

- Removed the Roast Scorer: the always-run `roast` CI job, its `ci-success` dependency, the `RoastScorer` quality check, `scripts/roast-scorer.sh`, `examples/roast-scorer.rs`, and the `cargo roast` alias. It was a heuristic score that re-ran Clippy, coverage, audit, deny, MSRV, builds, and benchmarks, so it added latency and false-failure risk without a deterministic signal. Deterministic quality gates remain the only merge criteria.
- Removed the CI write-back to `main`: the default workflow no longer declares `contents: write` and no longer commits `.agents/ci/ci-status.json`, `.agents/ci/ci-summary.md`, or benchmark history. The run summary is written to `$GITHUB_STEP_SUMMARY` and status files are uploaded as artifacts.
- Removed the ineffective `CiStatusArtifact` quality check, which passed vacuously when the file was absent and could only ever read a prior run's snapshot.
- Removed root `[features]` demo block (`cli`, `persistence`, `parallel`, `wasm`, `tracing-json`, `tracing-opentelemetry`) and optional `tokio` dependency from the root crate.
- Removed `redb` optional dependency and `kv` feature from `hybrid-storage-template`. Template users relying on `--features kv` should add `redb` directly to their project.

### Fixed

- Made `xtask`'s doctest check run `cargo test --workspace --doc --all-features`, matching CI. A restricted `default-members` list no longer hides a doctest failure in another workspace member.
- Made the `ci-success` aggregate fail closed: `changes` now feeds the evaluator, `success` is accepted, `skipped` is accepted only when the same path-filter contract proves the job was not required, and missing, `failure`, `cancelled`, `timed_out`, `action_required`, or unknown results fail the merge gate.
- Stopped skipping the compile/test chain for changes outside the well-known workspace directories. `ChangedPaths::classify` only evaluated its manifest check inside a directory-prefix guard, so a `Cargo.toml` in a directory such as `tools/tool/` was not classified as a code change. The `when_changed` globs matched Rust sources and lockfiles by directory prefix too, so `tools/tool/src/main.rs` and `crates/sample-app/Cargo.lock` selected no checks at all. Both now match on file shape (`**/*.rs`, `**/Cargo.toml`, `**/Cargo.lock`), which also covers nested lockfiles. `do-harness.toml` follows the same rule.
- Removed the unpatchable `rustls-webpki 0.102.8` dependency edge instead of suppressing its advisories. `hybrid-storage-template` now depends on `libsql` with `default-features = false, features = ["core"]`, which drops libsql's default `tls` feature and with it the whole `hyper-rustls 0.25` → `rustls 0.22` → `rustls-webpki 0.102.8` stack. `cargo audit` and `cargo deny check` now pass with an empty advisory ignore list.
- Restored the `when-changed` applicability globs for the `quality-gates` sensor in `do-harness.toml`. `"Cargo.toml"` covers the workspace root manifest and `"**/Cargo.toml"` covers workspace member manifests (inside and outside `crates/`); bare wildcards are avoided so Markdown-only edits do not trigger verification.
- Gave the `quality-gate` CI job a static name (`Quality Gate (pull-request)`). The previous name interpolated a `${{ }}` expression, which GitHub only evaluates when the job runs — a skipped job reported the raw expression, so the branch ruleset could never match its required status check and the PR was blocked from merging with no failing check to explain why. This affected every pull request that skipped the job (documentation-only and metrics-only changes). `scripts/validate-workflows.sh` now rejects an expression-based name for that job.

### Security

- (none)

[Unreleased]: https://github.com/your-org/your-repo/compare/v0.0.0...HEAD
