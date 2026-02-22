# Skill: release-rust

## Purpose
Create and publish a new release of the Rust project.

## Prerequisites
- All CI checks pass on main
- `CHANGELOG.md` updated
- `cargo-dist` installed

## Steps

### 1. Pre-release checks
```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
cargo audit && cargo deny check
```

### 2. Bump version in Cargo.toml
```bash
cargo update --workspace
```

### 3. Commit and tag
```bash
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "chore: release vX.Y.Z"
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin main --tags
```

### 4. GitHub Release via CI
Pushing a tag triggers `.github/workflows/release.yml`:
- Builds binaries for all targets
- Creates GitHub Release with assets
- Optionally publishes to crates.io

## Version Scheme
- `MAJOR.MINOR.PATCH` (Semantic Versioning)
- Breaking = MAJOR, features = MINOR, fixes = PATCH

## Success Criteria
- Tag on GitHub
- Release with binaries created
- CHANGELOG updated

## References
- [cargo-dist](https://opensource.axo.dev/cargo-dist/)
- [Keep a Changelog](https://keepachangelog.com/)
