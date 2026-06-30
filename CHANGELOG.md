# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- for new features.

### Changed

- `setup-rust` composite action now exposes a `cache-on-failure` input
  (default `true`); the Clippy job in CI overrides it to `false` so a
  failed run cannot re-upload poisoned `.fingerprint/` metadata to the
  rust-cache. Other jobs (Format, Test, Security, deny, Benchmarks, MSRV,
  Version-check) keep the safe `true` default.
- `examples/roast-scorer.rs` now uses `Result<(), Box<dyn std::error::Error>>`-
  returning `main` with `?`-propagation (no `.expect()` calls) and
  `#![allow(missing_docs)]`, replacing the pre-merge version on
  `origin/main` that violated `clippy::expect_used` and `missing_docs`
  under `-D warnings`. Behaviour for callers invoking
  `cargo run --example roast-scorer` is unchanged.

### Deprecated

- for soon-to-be removed features.

### Removed

- for now removed features.

### Fixed

- for any bug fixes.

### Security

- in case of vulnerabilities.
