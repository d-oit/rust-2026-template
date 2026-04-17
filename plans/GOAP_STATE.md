# GOAP World State
# Current project state from an AI agent's perspective

## State Variables (Executable Truth)

### Code Quality
- `is_linted`: true (CI — clippy passes with zero warnings, `-D warnings`)
- `is_formatted`: true (CI — `cargo fmt --all -- --check` passes)
- `has_zero_warnings`: true (CI — clippy all targets, all features)
- `is_semver_compliant`: false (not checked yet — run `cargo semver-checks`)

### Testing
- `has_unit_tests`: true (`example-crate` and `sample-app` both have `#[cfg(test)]` modules)
- `has_integration_tests`: false
- `tests_passing`: true (CI — `cargo nextest run --workspace --profile ci` passes)
- `has_doc_tests`: true (`example-crate::greet` has a doc test)
- `coverage_meets_target`: false (target: 80%, not measured)

### Crates
- `example_crate`: library placeholder — `greet(name) -> String`
- `sample_app`: binary — tokio, clap, serde, tracing, thiserror; `--count`, `--verbose`, `--config` flags

### Documentation
- `has_context_yaml`: true (`docs/architecture/context.yaml`)
- `has_agents_md`: true
- `is_architecture_documented`: true (ADR 0001)
- `readme_current`: true (updated to reflect two crates and all scripts)

### Git
- `is_dirty`: false
- `on_main`: true

## Current Phase
`Phase 0: Initialization` — CI pipeline verified and working, documentation updated

## Active Blockers
- None

## Recent Changes
- Added `sample-app` binary crate (tokio, clap, serde, tracing, thiserror)
- Added `scripts/release-manager.sh`
- Fixed CI: installed mold linker for clippy, test, and MSRV jobs
- Fixed CI: disabled sccache for cargo-deny job (runs in Docker)
- Updated toolchain from 1.85 to 1.87
- Updated all documentation to reflect current codebase
