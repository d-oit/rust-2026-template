# Changelog Template

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
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

## [0.3.3] - 2026-07-01

### Added

- for new features.

### Changed

- `CHANGELOG.md` reset to a clean skeleton (placeholder text only); all
  template-specific changelog entries now live exclusively in this file
  (`.template/CHANGELOG-TEMPLATE.md`). The `VERSION` file stays at `0.0.0`
  as the derived-project starter value.
- `scripts/bump-version.sh`: aligned the `[Unreleased]` skeleton block to
  the full 6-section Keep-a-Changelog format (Added/Changed/Deprecated/
  Removed/Fixed/Security) matching `CHANGELOG.md`; switched from literal
  `\n` to `printf` for BSD/macOS sed compatibility; changed the new-version
  diff link to `/compare/` format (Keep-a-Changelog standard) instead of
  `/releases/tag/`; replaced the hardcoded `d-oit/rust-2026-template` URL
  with a `your-org/your-repo` placeholder for derived-project genericness;
  fixed duplicate step numbers in the header comment.
- `scripts/init-template.sh`: added `release.toml`, `scripts/bump-version.sh`,
  and `CHANGELOG.md` to the list of files whose placeholder URLs get
  rewritten during template initialization.
- `setup-rust` composite action now exposes a `cache-on-failure` input
  (default `true`); the Clippy job in CI overrides it to `false` so a
  failed run cannot re-upload poisoned `.fingerprint/` metadata to the
  rust-cache. Other jobs (Format, Test, Security, deny, Benchmarks, MSRV,
  Version-check) keep the safe `true` default.
- `examples/roast-scorer.rs` uses `Result<(), Box<dyn std::error::Error>>`-
  returning `main` with `?`-propagation (no `.expect()` calls) and
  `#![allow(missing_docs)]` so the example compiles under
  `cargo clippy --all-targets -- -D warnings` (which enforces
  `clippy::expect_used` and `missing_docs`). Behaviour for callers
  invoking `cargo run --example roast-scorer` is unchanged.
- `hybrid-storage-template`: the `redb` KV backend is now opt-in via
  `--features kv` (it was previously always compiled because `redb` was a
  non-optional dependency). To restore prior compile parity, add `kv`
  to your default features in `Cargo.toml`. The `sqlite` feature
  (`libsql` backend) remains on by default.
- `hybrid-storage-template`: dropped unused direct dependencies
  `serde`, `serde_json`, and `tracing` from the manifest.

### Deprecated

- for soon-to-be removed features.

### Removed

- `[workspace.dependencies].rmcp` — the `rmcp` MCP framework was not used
  in any in-tree code path (only in a documentation banner comment). The
  `crates/mcp-server-template` crate itself remains and now uses raw
  `tokio`/`serde` for its server implementation.
- `fuzz/Cargo.toml`: dropped the unused `rust-2026-template = { path = ".." }`
  dependency; the fuzz targets only sample the workspace's `sample-app`.
- No-op `mcp` feature flag removed from root `Cargo.toml` — it had no
  `#[cfg(feature = "mcp")]` in any source file and only aliased `cli`.
  Stale `rmcp` references cleaned from `Cargo.toml` comments,
  `mcp-server-template/src/lib.rs` doc diagram, `README.md`, and
  `docs/src/getting-started.md` feature tables.
- Stale `RUSTSEC-2024-0436` advisory ignore removed from `deny.toml`
  (`paste` is no longer a transitive dependency after `rmcp` removal).

### Fixed

- `docs/src/architecture.md`: corrected crate name `hello-world-example` →
  `hello_world` to match the actual directory name.

### Security

- in case of vulnerabilities.

---

## [0.3.2] - 2026-06-17

### Added

- `/update-template-changelog` custom command for automated changelog updates from git history.

---

## [0.3.1] - 2026-06-17

### Added

- `verify-actions` skill with evals for CI action SHA verification.
- Pre-commit hook with line count check (max 500 LOC) and cargo env sourcing.
- `swatinem/rust-cache` and concurrency control to `fuzz.yml` and `release.yml`.

### Changed

- Unified `actions/checkout` to v6 and `upload-artifact` to v7 across all workflows.
- Extracted `sample-app` tests to `src/tests.rs` to pass LOC gate.

### Fixed

- Documentation corrections: dora-metrics workflow ref, context.yaml placeholders, ci.md tiers.
- README version badge, crate list, feature defaults, and benchmarks updated.
- Replaced `temp_dir` with current directory in test to satisfy Codacy.

---

## [0.3.0] - 2026-06-17

### Added

- `issue-triage` skill for reading open issues (GitHub/GitLab), categorizing by type and effort, and coordinating batch implementation.
- GitLab support to `atomic-commit` and `self-fix-loop` skills.
- GitLab platform auto-detection in `issue-triage` skill.
- Folder symlinks for `.claude/skills/` and `.qwen/skills/` for multi-CLI agent support.
- Cross-repo agent context via `.agents/context/` with `external-repos.json` and `shared-conventions.md`.

### Fixed

- `MIGRATION.md` version references corrected.
- Removed unused `async-trait` dependency.
- Moved `CHANGELOG-TEMPLATE` to `.template/` directory.
- Poisoned mutex handling in `MockBackend` for MCP security hardening.
- Clippy `significant-drop-tightening` warning resolved.
- Markdown lint issues and missing skill references resolved.

---

## [0.2.3] - 2026-06-08

### Added

- `scripts/generate-skills-md.sh` to auto-generate SKILLS.md from skill frontmatter.
- Auto-generated `.agents/SKILLS.md` with skill index table.

---

## [0.2.2] - 2026-06-02

### Added

- `.gitignore` guardrail rules to block root dummy/test/data files.
- `schema/` directory for JSON Schema definitions with reference example.
- `config/profiles/` pattern for profile-based runtime configuration.
- Upgraded `.codacy.yml` to full `.codacy/` directory for richer tool configuration.
- `reports/` directory convention for generated HTML review and analysis output.
- Workspace layering convention (documented in `docs/architecture.md` and `Cargo.toml`).
- `example-storage-pattern` crate demonstrating trait-only storage layer.
- `example-registry-pattern` crate demonstrating extensible handler dispatch.
- Automated documentation toolchain using `cargo-sync-readme` and `cargo-doc2readme`.
- `Makefile` with `docs`, `docs-check`, and `ci` targets.
- `docs-check.yml` CI workflow for automated documentation enforcement.

### Changed

- `README.md` now automatically synced from `src/lib.rs` crate-level documentation.

---

## [0.2.1] - 2026-05-31

### Added

- `validate-agents` CI job with entrypoint validation script.
- `agent-doc-flow.md` documentation for assistant entrypoint model.
- ADR 0002: CI optimization and agent validation.

### Changed

- Rust toolchain bumped from 1.87 to 1.88.
- CI jobs gated on file changes via `dorny/paths-filter`.
- CI status handling for skipped jobs via `netlify/ci-status`.
- `rmcp` dependency bumped from 0.3.2 to 1.7.0.
- `criterion` dependency bumped from 0.5.1 to 0.8.2.
- `actions/checkout` bumped from 4.3.1 to 6.0.2.
- `actions/setup-python` bumped from 5.6.0 to 6.2.0.
- `gitleaks/gitleaks-action` bumped from v2 to v3.
- `peter-evans/create-pull-request` bumped from 6.1.0 to 8.1.1.
- `fuzz.yml` and `mutants.yml` updated for path-gated triggers.
- `CHANGELOG.md` reset to clean template skeleton.

### Fixed

- Clippy `uninlined-format-args` lint under rustc 1.88.
- Documentation badges and MSRV references updated across all docs.

---

## [0.2.0] - 2026-05-29

### Added

- `VERSION` file at repo root as plain-text single source of truth for cross-tooling scripts and CI.
- `llms-full.txt` with complete source context for AI assistants (auto-generated from key docs).
- `scripts/generate-llms-txt.sh` to regenerate `llms.txt` and `llms-full.txt` after architectural changes.
- `benchmarks/` workspace crate with `end_to_end` and `memory_usage` Criterion benchmark suites.
- `fuzz/` scaffold with `cargo-fuzz` targets and weekly fuzz CI workflow (`.github/workflows/fuzz.yml`).
- Composable feature flags: `cli`, `persistence`, `parallel`, `wasm` with opt-in heavy dependencies.
- `[package]` section in root `Cargo.toml` with `include` manifest for crates.io publishing.
- CI jobs: benchmarks compile-check, VERSION consistency check, LLM context staleness check.

### Changed

- Workspace resolver upgraded from v2 to v3 (Rust 2024 edition default).
- Release profile: `lto = "fat"` (was `thin`), `panic = "unwind"` documented, `strip = "symbols"`.
- Clippy lints: ADR-driven phased strategy with promoted pedantic/nursery lints at warn.
- Redundant `#![forbid(unsafe_code)]` / `#![warn(clippy::pedantic)]` removed from source; enforced via `Cargo.toml` `[lints]`.
- Version bumped from 0.1.2 to 0.2.0.

### Fixed

- Removed unstable `wrap_comments = true` from `rustfmt.toml` (nightly-only).
- Stale `include` paths (`/benches`) and `exclude_re` (`^src/generated/`) in `Cargo.toml`.
- Unused `anyhow` and `insta` deps removed from `sample-app`.
- Unquoted `$EXCLUDE_DIR` in `scripts/quality-gates.sh` (ShellCheck).
- `MPL-2.0` added to `deny.toml` allowed licenses for `colored` optional dependency.

---

## [0.1.2] - 2026-05-19

### Added

- Codecov configuration for coverage gate enforcement.
- `cargo-dist` and `release.toml` for automated release engineering.
- `llms.txt` and `llms-full.txt` as standard LLM context files.
- `.test-quality.toml` for test quality enforcement (Step 10 in quality gates).
- `.shellcheckrc` and shell script quality gate integrated into pre-commit and CI.
- Enhanced `scripts/quality-gates.sh` with test quality and coverage checks.

### Fixed

- Empty link in `README.md` version badge causing markdownlint (MD042) failure.

---

## [0.1.1] - 2026-05-16

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
- `serde` pinned to `=1.0.194`, `serde_json` to `=1.0.143`, `insta` to `=1.47.2`
- CI: sccache disabled for `cargo-deny` job (runs in Docker without sccache)
- CI: mold linker installed for `clippy`, `test`, and `msrv` jobs

### Fixed

- Missing doc comments on `AppError` enum variants
- CLI short flag conflict (`-c` for `--count`)
- Clippy warnings: `single_match_else`, `uninlined_format_args`, `needless_borrows_for_generic_args`
- `Unicode-3.0` added to allowed licenses in `deny.toml`
- Potential DoS vector in JSON parsing by enforcing recursion limits.
- Log injection vulnerability by filtering Unicode Bidirectional (Bidi) control characters.

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

[Unreleased]: https://github.com/d-oit/rust-2026-template/compare/v0.3.3...HEAD
[0.3.3]: https://github.com/d-oit/rust-2026-template/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/d-oit/rust-2026-template/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/d-oit/rust-2026-template/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/d-oit/rust-2026-template/compare/v0.2.3...v0.3.0
[0.2.3]: https://github.com/d-oit/rust-2026-template/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/d-oit/rust-2026-template/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/d-oit/rust-2026-template/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/d-oit/rust-2026-template/compare/v0.1.2...v0.2.0
[0.1.2]: https://github.com/d-oit/rust-2026-template/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/d-oit/rust-2026-template/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/d-oit/rust-2026-template/releases/tag/v0.1.0
