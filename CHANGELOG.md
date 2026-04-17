# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial project structure

### Changed
- Updated toolchain from 1.85 to 1.87
- Updated tokio features from "full" to specific features (rt-multi-thread, macros, sync, time, fs)

### Deprecated

### Removed

### Fixed
- Added missing documentation for enum variants in sample-app
- Fixed CLI argument short name conflict (-c)
- Fixed clippy warnings: single_match_else, uninlined_format_args, needless_borrows_for_generic_args
- Added mold linker installation for clippy and MSRV CI jobs
- Added Unicode-3.0 license to allowed licenses

### Security

---

## [0.1.0] - 2025-01-01

### Added
- Initial release from rust-2026-template
- Cargo workspace structure
- CI/CD with GitHub Actions (fmt, clippy, nextest, audit, deny)
- Release workflow with cargo-dist
- Agent guidelines (AGENTS.md, CLAUDE.md, GEMINI.md)
- Agent skills (.agents/skills/)
- Supply chain security (deny.toml, cargo-audit)
- WSL2 optimizations (.cargo/config.toml, .vscode/settings.json)
- Rust 2024 edition formatting (rustfmt.toml)
- Clippy configuration (.clippy.toml)

[Unreleased]: https://github.com/OWNER/REPO/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/OWNER/REPO/releases/tag/v0.1.0
