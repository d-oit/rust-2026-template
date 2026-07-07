# GOAP World State

# Current project state from an AI agent's perspective

## State Variables (Executable Truth)

### Code Quality

- `is_linted`: true
- `is_formatted`: true
- `has_zero_warnings`: true
- `all_files_under_500_loc`: true
- `is_semver_compliant`: false (not checked yet)

### Testing

- `has_unit_tests`: true
- `has_integration_tests`: false
- `tests_passing`: true
- `has_doc_tests`: true
- `coverage_meets_target`: false (target: 80%, not measured)

### CI

- `ci_incremental_disabled`: true (PR #243, CARGO_INCREMENTAL=0)
- `crossbeam_epoch_updated`: true (0.9.18→0.9.20, RUSTSEC-2026-0204)
- `cargo_machete_clean`: true (false-positive ignores added)
- `fast_dev_profile_combined`: true (debug=0 + strip + panic=abort)
- `faster_builds_docs_merged`: true (PR #242)

### Git

- `is_dirty`: false
- `on_main`: true
- `prs_closed`: PR #242, PR #243 merged

## Current Phase

`Phase: PR Merge & CI Fixup — Complete`

## Active Blockers

- None

## Recent Changes

- Merged PR #243: Disable incremental compilation in CI
- Merged PR #242: Faster Builds guide + fast-dev profile reconciliation
- Fixed pre-existing: crossbeam-epoch vulnerability (RUSTSEC-2026-0204)
- Fixed pre-existing: checkpoint-template LOC (504→497, under 500 limit)
- Fixed pre-existing: cargo-machete false positives (optional/feature-gated deps)
