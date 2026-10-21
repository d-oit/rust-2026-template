# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- for new features.

### Changed

- Promoted `pedantic` and `nursery` clippy lint groups from `allow` to `warn` at workspace level. Individual crates use local `[lints.clippy]` overrides where needed (Cargo 1.88 limitation).
- Expanded `tokio` features in `checkpoint-template` to include `rt-multi-thread`, `macros`, and `sync` (needed for `#[tokio::test]`).

### Deprecated

- (none)

### Removed

- Removed root `[features]` demo block (`cli`, `persistence`, `parallel`, `wasm`, `tracing-json`, `tracing-opentelemetry`) and optional `tokio` dependency from the root crate.
- Removed `redb` optional dependency and `kv` feature from `hybrid-storage-template`. Template users relying on `--features kv` should add `redb` directly to their project.

### Fixed

- Removed the unpatchable `rustls-webpki 0.102.8` dependency edge instead of suppressing its advisories. `hybrid-storage-template` now depends on `libsql` with `default-features = false, features = ["core"]`, which drops libsql's default `tls` feature and with it the whole `hyper-rustls 0.25` → `rustls 0.22` → `rustls-webpki 0.102.8` stack. `cargo audit` and `cargo deny check` now pass with an empty advisory ignore list.
- Restored the `when-changed` applicability globs for the `quality-gates` sensor in `do-harness.toml`. `"Cargo.toml"` covers the workspace root manifest and `"**/Cargo.toml"` covers workspace member manifests (inside and outside `crates/`); bare wildcards are avoided so Markdown-only edits do not trigger verification.

### Security

- (none)

[Unreleased]: https://github.com/your-org/your-repo/compare/v0.0.0...HEAD
