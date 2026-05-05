# GOAP World State

# Current project state from an AI agent's perspective

## State Variables (Executable Truth)

### Code Quality

- `is_linted`: true (CI — clippy passes with zero warnings, -D warnings)
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
- `has_agents_md`: true (canonical instruction file)
- `is_architecture_documented`: true (ADR 0001)
- `readme_current`: true (human-facing entry point, reflects crates and scripts)
- `docs_separated`: true (clear split between human and agent documentation)

### Git

- `is_dirty`: false
- `on_main`: true

## Current Phase

`Phase 0: Initialization` — Documentation overhauled for human/agent clarity

## Active Blockers

- None

## Recent Changes

- Overhauled documentation structure: README.md for humans, AGENTS.md for agents
- Simplified agent reference files (CLAUDE.md, GEMINI.md, etc.)
- Added CI/linting rules to AGENTS.md and conventions.md to prevent regressions
- Fixed CI failures: MD031 markdown violation and commit message body length
