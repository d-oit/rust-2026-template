# Skill: release-rust

## Purpose
Create and publish a new release of the Rust project.

## Prerequisites
- All CI checks pass on main
- `CHANGELOG.md` updated
- `cargo-dist` installed

## Steps

### 0. Verify crate name availability on crates.io (FIRST TIME PUBLISH ONLY)

Before the very first publish, confirm the crate name is not already taken:

```bash
# Check via cargo search
cargo search <your-crate-name>

# Or via API (404 = available, 200 = taken)
curl -s https://crates.io/api/v1/crates/<your-crate-name> | python3 -m json.tool | grep '"name"'

# Or open in browser: https://crates.io/crates/<your-crate-name>
```

> See `.agents/skills/crates-io-name-check/SKILL.md` for full naming guidance and best practices.

**Do not proceed with publishing if the name is taken — choose a unique name first.**

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
- Crate name verified unique on crates.io (first publish)

## References
- [cargo-dist](https://opensource.axo.dev/cargo-dist/)
- [Keep a Changelog](https://keepachangelog.com/)
- [crates.io naming policy](https://crates.io/policies)
