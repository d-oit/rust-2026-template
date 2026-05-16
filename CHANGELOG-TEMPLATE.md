# Changelog Template

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- for new features.

### Changed

- for changes in existing functionality.

### Deprecated

- for soon-to-be removed features.

### Removed

- for now removed features.

### Fixed

- for any bug fixes.

### Security

- in case of vulnerabilities.

---

## [0.1.1] - 2026-01-01

### Added

- Hardened `sample-app` configuration loading with size limits and UTF-8 sanitization.
- Integrated `cargo-mutants` for automated mutation testing in CI.
- Added lookup-table optimizations for string formatting in performance-critical loops.

### Changed

- Toolchain upgraded to Rust 1.87 with 2024 edition support.
- Dependency pinning for `serde` and `insta` to mitigate supply chain risks.

### Fixed

- Resolved potential DoS vector in JSON parsing by enforcing recursion limits.
- Fixed log injection vulnerability by filtering Unicode Bidirectional (Bidi) control characters.
