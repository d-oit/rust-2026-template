# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- CI path-based gating via `dorny/paths-filter` to skip jobs on unrelated changes (#112)
- `validate-agents` CI job to check assistant entrypoint files follow the `@AGENTS.md` directive model (#112)
- Agent entrypoint validation script `scripts/validate-agent-entrypoints.sh` (#112)
- `agents-docs/agent-doc-flow.md` documenting the single-source-of-truth agent instruction model (#112)
- Architecture Decision Record `plans/adr/0002-ci-optimization-and-agent-validation.md` (#112)

### Changed

- Bumped `rmcp` from 0.3.2 to 1.7.0 (requires rustc 1.88+) (#120)
- Bumped `criterion` from 0.5.1 to 0.8.2 (#117)
- Bumped `actions/checkout` from 4.3.1 to 6.0.2 (#114)
- Bumped `actions/setup-python` from 5.6.0 to 6.2.0 (#118)
- Bumped `gitleaks/gitleaks-action` from v2 to v3 (Node 20 → 24 runtime) (#115)
- Bumped `peter-evans/create-pull-request` from 6.1.0 to 8.1.1 (#116)
- Bumped pinned Rust toolchain from 1.87 to 1.88 in `rust-toolchain.toml` (#120)
- CI jobs are now conditional on file-change detection for faster PR feedback (#112)
- CI status reporting handles skipped jobs gracefully (#112)
- Updated `fuzz.yml` and `mutants.yml` workflows for path-gated triggers (#112)
- Updated documentation badges and MSRV references from 1.87 to 1.88

### Fixed

- Clippy `uninlined-format-args` lint in `examples/hello_world/src/main.rs` under rustc 1.88 (#120)

[Unreleased]: https://github.com/d-oit/rust-2026-template/compare/v0.0.0...HEAD
