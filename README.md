# Rust 2026 Template

[![CI](https://github.com/d-oit/rust-2026-template/actions/workflows/ci.yml/badge.svg)](https://github.com/d-oit/rust-2026-template/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust 1.87+](https://img.shields.io/badge/rust-1.87%2B-orange.svg)](https://www.rust-lang.org)

A production-ready GitHub repository template for modern Rust development in 2026. This template provides a robust workspace foundation with pre-configured CI/CD, quality gates, security auditing, and native support for AI-assisted development.

## Who is this for?

- **Developers** starting new production-grade Rust projects.
- **Teams** wanting to enforce consistent quality standards across multiple crates.
- **Maintainers** looking for a template with batteries-included CI/CD and security tooling.
- **AI-Native Engineers** who want their repository to be immediately understandable by coding agents.

## Key Features

- **Modern Workspace Layout** — Multi-crate Cargo workspace using the `crates/` convention.
- **Rust 2024 Edition** — Pre-configured for the latest language features with a pinned MSRV (1.87+).
- **Strict Quality Gates** — Local and CI-enforced checks for formatting, linting, and testing.
- **Supply Chain Security** — Integrated dependency auditing and license policy enforcement.
- **Performance Optimized** — Configured for fast builds (mold linker) and efficient development.
- **AI Agent Native** — First-class support for AI coding assistants with structured guidance and skill runbooks.

## Included Tooling

This template integrates best-in-class tools from the Rust ecosystem:

- **Testing**: `cargo-nextest` for faster, cleaner test execution and `proptest` for property-based testing.
- **Linting**: High-severity `clippy` rules enabled by default in `.clippy.toml`.
- **Security**: `cargo-audit` for vulnerability scanning and `cargo-deny` for dependency policy management.
- **CI/CD**: Comprehensive GitHub Actions workflows for continuous integration and automated releases via `cargo-dist`.
- **Scripts**: Helper scripts in `scripts/` for local development and release management.

## Repository Structure

```text
.
├── .agents/             # AI agent skill definitions and runbooks
├── .cargo/              # Cargo configuration (linker, profiles, aliases)
├── .github/             # CI/CD workflows and GitHub templates
├── crates/              # Workspace crates (libraries and applications)
│   ├── example-crate/   # Template library crate
│   └── sample-app/      # Template binary application
├── docs/                # Project documentation
├── plans/               # Architecture Decision Records (ADR) and roadmap
├── scripts/             # Development and quality gate scripts
├── AGENTS.md            # Canonical instructions for AI coding agents
└── Cargo.toml           # Workspace manifest
```

## Quick Start

1. **Use this template**: Click the "Use this template" button on GitHub to create your new repository.
2. **Setup**: Follow the detailed guide in **[QUICKSTART.md](QUICKSTART.md)** to rename crates and configure metadata.
3. **Verify**: Run the quality gates to ensure everything is set up correctly:
   ```bash
   bash scripts/quality-gates.sh
   ```

## CI/CD and Quality Gates

The project enforces a "zero-warning" policy. The CI pipeline runs on every push and pull request, covering:

- **Format & Lint**: `cargo fmt` and `cargo clippy`.
- **Test**: Workspace-wide testing with `cargo nextest`.
- **Security**: Dependency vulnerability audit and license compliance.
- **MSRV**: Verification against the Minimum Supported Rust Version.

## AI Agent Integration

This repository is designed to be "Agent-Native." It includes `AGENTS.md` as a canonical source of truth for AI coding assistants (like Claude Code, Gemini CLI, or GitHub Copilot). These instructions help agents understand project conventions, use available tools correctly, and maintain high code quality.

## Customization Guidance

- **Renaming**: Use the instructions in `QUICKSTART.md` to rename the placeholder crates in `crates/`.
- **Lints**: Adjust `.clippy.toml` if you need to customize linting rules.
- **Security**: Update `deny.toml` to match your organization's license and dependency policies.

## Maintenance

- **Update Toolchain**: Update `rust-toolchain.toml` to bump the required Rust version.
- **Dependency Updates**: Dependabot is configured to keep your dependencies up to date.

## Documentation

- [QUICKSTART.md](QUICKSTART.md) — Initial setup and configuration.
- [CONTRIBUTING.md](CONTRIBUTING.md) — Guidelines for contributing to the project.
- [SECURITY.md](SECURITY.md) — Security policy and vulnerability reporting.
- [AGENTS.md](AGENTS.md) — Canonical instructions for AI agents.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
