# Template Profiles Integration Tests

This directory contains fixture-based integration tests for `rust-2026-template` project profiles.

## Running Tests

Execute the profile test script:

```bash
bash tests/template_profiles/test_profiles.sh
```

## What Is Tested

1. **Schema Validation:** Ensures all 6 shipped profile blueprints (`minimal`, `library`, `cli`, `service`, `workspace`, `ai-agent`) parse and pass structural validation.
2. **Inspection Command:** Verifies `cargo xtask template inspect` prints human-readable profile details.
3. **Dry-Run Mode:** Verifies dry-run initialization previews changes without making any edits on disk.
4. **Workspace Generation & Buildability:** Initializes each profile in a clean temporary clone and verifies that:
   - `cargo check --workspace` succeeds.
   - `cargo test --workspace` succeeds.
   - Profile-specific `default_tier` is written into `config/xtask.json`.
   - Workspace-shaping decisions (like removing unselected pattern crates, benchmarks, fuzz targets, or workflows) are correctly enforced.
