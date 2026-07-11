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

- Added `.cargo/audit.toml` to ignore unfixable `rustls-webpki 0.102.8` transitive advisories (RUSTSEC-2026-0049/0099/0104/0098), blocked upstream by `libsql 0.9` → `rustls 0.22`.

### Security

- (none)

[Unreleased]: https://github.com/your-org/your-repo/compare/v0.0.0...HEAD
