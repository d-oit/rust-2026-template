# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- for new features.

### Changed

- `hybrid-storage-template`: the `redb` KV backend is now opt-in via `--features kv`
  (it was previously always compiled because `redb` was a non-optional dependency).
  Downstream consumers who want the prior compile parity should enable the `kv`
  feature explicitly. `libsql` (`sqlite` feature) remains on by default.
- `hybrid-storage-template`: dropped unused direct dependencies `serde`, `serde_json`,
  and `tracing` from the manifest. The crate's public API does not reference any of
  these symbols, so consumers no longer pay the transitive cost.

### Deprecated

- for soon-to-be removed features.

### Removed

- `crates/mcp-server-template` and the workspace `[workspace.dependencies].rmcp`
  entries. The `rmcp` MCP framework was not used in any in-tree code path
  (it appeared only in a documentation banner comment). Enable the root
  `--features cli` for the demo CLI instead.
- `fuzz/Cargo.toml`: dropped the unused `rust-2026-template = { path = ".." }`
  dependency. The fuzz targets only sample the workspace's `sample-app` crate.

### Fixed

- for any bug fixes.

### Security

- in case of vulnerabilities.
