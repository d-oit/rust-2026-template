# Rust 2026 Template

[![CI](https://github.com/d-oit/rust-2026-template/actions/workflows/ci.yml/badge.svg)](https://github.com/d-oit/rust-2026-template/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/d-oit/rust-2026-template/branch/main/graph/badge.svg)](https://codecov.io/gh/d-oit/rust-2026-template)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust 1.87+](https://img.shields.io/badge/rust-1.87%2B-orange.svg)](https://www.rust-lang.org)
[![Mutation Testing](https://github.com/d-oit/rust-2026-template/actions/workflows/mutants.yml/badge.svg)](https://github.com/d-oit/rust-2026-template/actions/workflows/mutants.yml)

A high-performance, security-hardened GitHub repository template for modern Rust development. This template integrates the Rust 2024 edition, advanced CI/CD pipelines, and AI-native workflows to accelerate building robust applications.

## Overview

This template is designed for Rust developers who want to start new projects with best practices baked in. It provides a modular workspace structure, comprehensive quality gates, and built-in support for AI-assisted development.

## Key Features

- **Rust 2024 Edition:** Leverages the latest language features and idioms with an MSRV of 1.87.
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
├── .github/             # GitHub Actions workflows and issue templates
├── agents-docs/         # Detailed documentation for AI agents
├── crates/              # Workspace members (libraries and applications)
│   ├── example-crate/   # Placeholder library crate
│   └── sample-app/      # Reference application implementing best practices
├── scripts/             # Automation scripts for quality gates and releases
├── AGENTS.md            # Canonical instructions for AI coding agents
├── llms.txt             # LLM context file (machine-readable project overview)
├── Cargo.toml           # Workspace manifest
└── QUICKSTART.md        # Comprehensive setup guide
```

## CI/CD and Quality Gates

The project enforces high standards through a multi-layered verification process:

- **CI Pipeline:** Automatically runs formatting checks, Clippy lints, tests, security audits (`cargo-audit`), and supply chain checks (`cargo-deny`) on every PR.
- **Local Gates:** Run `./scripts/quality-gates.sh` before committing to mirror the CI checks locally.
- **Mutation Testing:** Periodic runs of `cargo-mutants` verify that your tests actually catch bugs.

## Customization Guidance

To adapt this template to your needs:

- **Renaming Crates:** Search and replace `example-crate` and `sample-app` with your desired crate names.
- **Adjusting Lints:** Modify `.clippy.toml` or crate-level attributes if you need to diverge from the default pedantic lint set.
- **Security Policy:** Review `deny.toml` to customize allowed licenses and dependency bans.

## Maintenance

Contributions are welcome! Please refer to **[CONTRIBUTING.md](CONTRIBUTING.md)** for guidelines on how to propose changes or report issues. Security vulnerabilities should be reported according to the process in **[SECURITY.md](SECURITY.md)**.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
