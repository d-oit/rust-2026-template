# Release Provenance Test Fixtures

This directory contains regression fixtures for verifying offline release provenance and `cargo-auditable` dependency metadata embedding.

## Fixtures

1. `auditable_release.elf`:
   A minimal Linux x86_64 ELF binary compiled with `cargo auditable build --release` containing embedded dependency metadata in the `.dep-v0` section.
2. `ordinary_cargo.elf`:
   A standard Linux x86_64 ELF binary compiled with default `cargo build --release` without `cargo-auditable`, lacking `.dep-v0` metadata.

## Verification

The offline regression suite `tests/release_provenance_test.sh` parses ELF, Mach-O, and PE headers to verify that:
- Auditable release binaries are detected and verified without requiring network access to vulnerability databases.
- Non-auditable binaries trigger clear remediation instructions directing maintainers to ensure `auditable = true` in `dist-workspace.toml`.
