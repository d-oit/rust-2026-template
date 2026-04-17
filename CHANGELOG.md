# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `sample-app` binary crate: tokio, clap, serde/serde_json, tracing, thiserror, anyhow
  - CLI flags: `--count`, `--verbose`, `--config`
  - JSON config loading with 1 MB size guard and `#[serde(deny_unknown_fields)]`
  - Typed `AppError` enum via `thiserror`
- `scripts/release-manager.sh`: dry-run release workflow (validate / prepare / publish)
- `scripts/code-quality.sh`: fmt | clippy | audit | check | fix operations
- Cargo aliases in `.cargo/config.toml`: `check-all`, `test-all`, `fmt-check`, `lint`, `audit-check`, `release-check`
- 9-step `scripts/quality-gates.sh` with `--fix` flag (adds unused-deps, privacy, and secret scans)
- 9 agent skills: `anti-ai-slop`, `build-rust`, `crates-io-name-check`, `lint-rust`, `privacy-first`, `release-rust`, `skill-creator`, `skill-evaluator`, `test-rust`
- `docs/architecture/context.yaml` for AI agent context
- `plans/GOAP_STATE.md` for AI agent world state tracking
- GitHub issue templates: `bug_report.yml`, `feature_request.yml`

### Changed
- Toolchain bumped from 1.85 to 1.87 (`rust-toolchain.toml` and `Cargo.toml` `rust-version`)
- `tokio` features narrowed from `"full"` to `rt-multi-thread, macros, sync, time, fs`
- `serde` pinned to `=1.0.194`, `serde_json` to `=1.0.140`, `insta` to `=1.40.0`
- CI: sccache disabled for `cargo-deny` job (runs in Docker without sccache)
- CI: mold linker installed for `clippy`, `test`, and `msrv` jobs

### Fixed
- Missing doc comments on `AppError` enum variants
- CLI short flag conflict (`-c` for `--count`)
- Clippy warnings: `single_match_else`, `uninlined_format_args`, `needless_borrows_for_generic_args`
- `Unicode-3.0` added to allowed licenses in `deny.toml`

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

[Unreleased]: https://github.com/d-oit/rust-2026-template/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/d-oit/rust-2026-template/releases/tag/v0.1.0
