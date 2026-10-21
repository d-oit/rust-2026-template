# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Added an `actionlint` quality check (`GitHub Actions Semantic Check`) that validates GitHub Actions workflows semantically: undefined expressions, `needs` references to non-existent jobs, invalid contexts, and cron syntax. It runs on the `pull-request`, `protected-branch`, and `release` tiers, is required on the protected tiers, and installs `actionlint` v1.7.12 in CI. Only actionlint's own checks run — its bundled shellcheck and pyflakes passes are disabled because the repository lints shell separately and several workflow `run:` blocks rely on intentional word splitting.
- Added a `yamllint` quality check (`YAML Lint`) that mirrors the `yamllint .` run the CI lint job already performed, so YAML style violations are caught by `./scripts/quality-gates.sh` instead of only in CI. It is required on the `protected-branch` and `release` tiers.

### Changed

- Promoted `pedantic` and `nursery` clippy lint groups from `allow` to `warn` at workspace level. Individual crates use local `[lints.clippy]` overrides where needed (Cargo 1.88 limitation).
- Expanded `tokio` features in `checkpoint-template` to include `rt-multi-thread`, `macros`, and `sync` (needed for `#[tokio::test]`).
- Bumped the pinned `do-harness` release in `scripts/setup-harness.sh` from `v0.2.1` to `v0.3.4`. The `do-harness.toml` sensor configuration format is unchanged between these versions.

### Deprecated

- (none)

### Removed

- Removed root `[features]` demo block (`cli`, `persistence`, `parallel`, `wasm`, `tracing-json`, `tracing-opentelemetry`) and optional `tokio` dependency from the root crate.
- Removed `redb` optional dependency and `kv` feature from `hybrid-storage-template`. Template users relying on `--features kv` should add `redb` directly to their project.

### Fixed

- Removed the unpatchable `rustls-webpki 0.102.8` dependency edge instead of suppressing its advisories. `hybrid-storage-template` now depends on `libsql` with `default-features = false, features = ["core"]`, which drops libsql's default `tls` feature and with it the whole `hyper-rustls 0.25` → `rustls 0.22` → `rustls-webpki 0.102.8` stack. `cargo audit` and `cargo deny check` now pass with an empty advisory ignore list.
- Restored the `when-changed` applicability globs for the `quality-gates` sensor in `do-harness.toml`. `"Cargo.toml"` covers the workspace root manifest and `"**/Cargo.toml"` covers workspace member manifests (inside and outside `crates/`); bare wildcards are avoided so Markdown-only edits do not trigger verification.
- Gave the `quality-gate` CI job a static name (`Quality Gate (pull-request)`). The previous name interpolated a `${{ }}` expression, which GitHub only evaluates when the job runs — a skipped job reported the raw expression, so the branch ruleset could never match its required status check and the PR was blocked from merging with no failing check to explain why. This affected every pull request that skipped the job (documentation-only and metrics-only changes). `scripts/validate-workflows.sh` now rejects an expression-based name for that job.

### Security

- (none)

[Unreleased]: https://github.com/your-org/your-repo/compare/v0.0.0...HEAD
