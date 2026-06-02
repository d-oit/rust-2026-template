# Rust 2026 Template

[![CI](https://github.com/d-oit/rust-2026-template/actions/workflows/ci.yml/badge.svg)](https://github.com/d-oit/rust-2026-template/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/d-oit/rust-2026-template/branch/main/graph/badge.svg)](https://codecov.io/gh/d-oit/rust-2026-template)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust 1.88+](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)
[![Mutation Testing](https://github.com/d-oit/rust-2026-template/actions/workflows/mutants.yml/badge.svg)](https://github.com/d-oit/rust-2026-template/actions/workflows/mutants.yml)
[![Template Version](https://img.shields.io/badge/version-0.2.1-blue)](CHANGELOG-TEMPLATE.md)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)

<!-- cargo-sync-readme start -->

A production-ready Rust workspace template with modern tooling, CI/CD,
and AI agent integration.

## Overview

This template is designed for Rust developers who want to start new projects with best practices baked in. It provides a modular workspace structure, comprehensive quality gates, and built-in support for AI-assisted development.

## Features

- **Rust 2024 Edition:** Leverages the latest language features and idioms with an MSRV of 1.88.
- **Workspace Layout:** Clean separation of concerns with a `crates/` directory for internal libraries and applications.
- **Security First:** Pre-configured supply chain audits, secret scanning, and hardened configuration patterns.
- **Performance Optimized:** Includes configurations for the `mold` linker and optimized development profiles.
- **AI-Native:** First-class support for AI coding agents with specialized skills and canonical instruction sets. Includes `llms.txt` for machine-readable project context.

## Example

```rust,no_run

let result = add(2, 3);
assert_eq!(result, 5);
```

<!-- cargo-sync-readme end -->

## Key Features

- **Rust 2024 Edition:** Leverages the latest language features and idioms with an MSRV of 1.88.
- **Workspace Layout:** Clean separation of concerns with a `crates/` directory for internal libraries and applications.
- **Security First:** Pre-configured supply chain audits, secret scanning, and hardened configuration patterns.
- **Performance Optimized:** Includes configurations for the `mold` linker and optimized development profiles.
- **AI-Native:** First-class support for AI coding agents with specialized skills and canonical instruction sets. Includes `llms.txt` for machine-readable project context.

## Included Tooling

- **Testing:** `cargo-nextest` for faster test execution and `proptest` for property-based testing.
- **Quality Assurance:** `cargo-mutants` for mutation testing and `clippy` with a zero-warnings policy.
- **CI/CD:** Multi-stage GitHub Actions for linting, testing, security audits, and automated releases.
- **Local Workflows:** Helper scripts for running the entire quality gate pipeline locally.

## Quick Start

1. **Use this Template:** Click the **"Use this template"** button on GitHub.
2. **Setup:** Follow the detailed instructions in **[QUICKSTART.md](QUICKSTART.md)**.
3. **Customize:** Rename the placeholder crates and update `Cargo.toml` metadata.
4. **Develop:** Use `./scripts/quality-gates.sh` to ensure your changes meet the project's quality standards.

## Repository Structure

```text
.
├── .agents/             # AI agent specialized skills and workflow definitions
├── .cargo/              # Cargo configuration (linker, profiles, aliases)
├── .codacy/             # Codacy static analysis and agent review configs
├── .github/             # GitHub Actions workflows and issue templates
├── agents-docs/         # Detailed documentation for AI agents
├── config/              # Profile-based runtime configuration
│   └── profiles/        # Environment-specific JSON configs (default.json, etc.)
├── crates/              # Workspace members (libraries and applications)
│   ├── example-crate/   # Placeholder library crate
│   └── sample-app/      # Reference application implementing best practices
├── reports/             # Generated HTML review and analysis output (ignored)
├── schema/              # JSON Schema definitions for config/API contracts
├── scripts/             # Automation scripts for quality gates and releases
├── AGENTS.md            # Canonical instructions for AI coding agents
├── llms.txt             # LLM context file (machine-readable project overview)
├── Cargo.toml           # Workspace manifest
└── QUICKSTART.md        # Comprehensive setup guide
```

## CI/CD and Quality Gates

The project enforces high standards through a multi-layered verification process:

- **CI Pipeline:** Automatically runs formatting checks, Clippy lints, tests, security audits (`cargo-audit`), supply chain checks (`cargo-deny`), benchmarks compile-check, and VERSION consistency checks on every PR.
- **Local Gates:** Run `./scripts/quality-gates.sh` before committing to mirror the CI checks locally.
- **Mutation Testing:** Periodic runs of `cargo-mutants` verify that your tests actually catch bugs.

## Feature Flags

The template demonstrates composable feature flags for pluggable backends:

| Feature | Description | Enabled by default |
|---------|-------------|-------------------|
| `cli`   | CLI binary support (clap, anyhow, colored) | Yes |
| `persistence` | Persistence backend (libsql) | Yes |
| `parallel` | CPU parallelism (rayon) | No |
| `wasm` | WASM build target support | No |
| `mcp` | MCP server support (requires cli) | No |
| `tracing-json` | JSON tracing output | No |
| `tracing-opentelemetry` | OpenTelemetry tracing backend | No |

## Benchmarks

The template provides a two-layer benchmark structure:

- **`benches/`**: Standard Criterion harnesses at the crate root
- **`benchmarks/`**: A separate workspace crate for complex, cross-crate benchmark suites

Run benchmarks with:

```bash
cargo bench -p benchmarks
```

## Fuzz Testing

A fuzz testing scaffold is included using `cargo-fuzz`:

```bash
# Install cargo-fuzz (nightly required)
cargo install cargo-fuzz

# Run fuzz targets
cargo fuzz run fuzz_parse_input -- -max_total_time=30
```

The fuzzer runs weekly via GitHub Actions (`.github/workflows/fuzz.yml`).

## AI Assistant Context Files

This template ships structured context files for AI coding assistants:

- **`llms.txt`**: Condensed project overview for token-efficient LLM context
- **`llms-full.txt`**: Complete source context for deep analysis (auto-generated)

Regenerate both files after significant architectural changes:

```bash
bash scripts/generate-llms-txt.sh
```

## Output Artifacts

The project uses several directories for generated artifacts and documentation:

- **`reports/`**: Standardized directory for generated HTML reports (coverage, audit, benchmarks). This directory is git-ignored by default.
- **`ci-status.json` / `ci-summary.md`**: Machine-readable and human-friendly CI health baselines.
- **`target/`**: Rust build artifacts.

## VERSION File

A `VERSION` file at the repo root serves as a plain-text single source of truth for tooling that can't easily parse TOML:

```bash
VERSION=$(cat VERSION)
echo "Building version $VERSION"
```

The CI pipeline verifies `VERSION` content matches `Cargo.toml` on every push to main.

## Customization Guidance

To adapt this template to your needs:

- **Renaming Crates:** Search and replace `example-crate` and `sample-app` with your desired crate names.
- **Adjusting Lints:** Modify `.clippy.toml` or crate-level attributes if you need to diverge from the default pedantic lint set.
- **Security Policy:** Review `deny.toml` to customize allowed licenses and dependency bans.

## Maintenance

Contributions are welcome! Please refer to **[CONTRIBUTING.md](CONTRIBUTING.md)** for guidelines on how to propose changes or report issues. Security vulnerabilities should be reported according to the process in **[SECURITY.md](SECURITY.md)**.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
