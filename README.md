# rust-2026-template

[![CI](https://github.com/d-oit/rust-2026-template/actions/workflows/ci.yml/badge.svg)](https://github.com/d-oit/rust-2026-template/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust 1.87+](https://img.shields.io/badge/rust-1.87%2B-orange.svg)](https://www.rust-lang.org)

Best practice 2026 Rust GitHub repository template with AI agent support, CI/CD, quality gates, and all modern Rust tooling.

## Features

- **Workspace structure** - Multi-crate Cargo workspace with `crates/` layout
- **2026 Rust edition** - Rust edition 2024, MSRV 1.87
- **CI/CD pipeline** - Full GitHub Actions CI with format, clippy, test, security audit, cargo-deny, MSRV check
- **AI agent support** - `AGENTS.md`, `CLAUDE.md`, `GEMINI.md` for AI coding assistants
- **Agent skills** - `.agents/skills/` directory with reusable agent skill files
- **Quality gates** - `scripts/quality-gates.sh` for local quality checks
- **Architecture decisions** - `plans/adr/` with ADR template
- **Supply chain security** - `deny.toml` with cargo-deny v2 configuration
- **Modern linting** - `.clippy.toml` with pedantic lints
- **Formatting** - `rustfmt.toml` with Rust 2024 edition settings
- **VSCode** - `.vscode/settings.json` with rust-analyzer and WSL2 support
- **cargo-nextest** - `.config/nextest.toml` with CI profile

## Quick Start

1. Click **"Use this template"** on GitHub to create a new repo
2. Replace `example-crate` under `crates/` with your own crate(s)
3. Update `Cargo.toml` workspace metadata (name, authors, repository, etc.)
4. Update `AGENTS.md`, `CLAUDE.md`, `GEMINI.md` with your project details
5. Push to `main` and watch CI pass!

## Repository Structure

```
rust-2026-template/
├── .agents/
│   └── skills/          # AI agent skill files (release, CI, etc.)
├── .cargo/
│   └── config.toml      # Cargo config (WSL2, sparse registry)
├── .config/
│   └── nextest.toml     # cargo-nextest profiles (default, ci)
├── .github/
│   ├── ISSUE_TEMPLATE/  # GitHub issue templates
│   ├── PULL_REQUEST_TEMPLATE.md
│   └── workflows/
│       ├── ci.yml           # Main CI pipeline
│       └── release.yml      # Release workflow
├── .vscode/
│   └── settings.json    # VSCode + rust-analyzer settings
├── crates/
│   └── example-crate/   # Replace with your crate(s)
│       ├── src/lib.rs
│       └── Cargo.toml
├── plans/
│   └── adr/             # Architecture Decision Records
├── scripts/
│   └── quality-gates.sh # Local quality gate runner
├── src/
│   └── lib.rs           # Root lib stub (for template reference)
├── .clippy.toml         # Clippy lint configuration
├── .gitignore           # Rust + WSL2 + IDE gitignore
├── AGENTS.md            # AI agent instructions (OpenAI Codex)
├── CHANGELOG.md         # Keep-a-changelog format
├── CLAUDE.md            # Claude Code instructions
├── Cargo.toml           # Workspace Cargo manifest
├── GEMINI.md            # Gemini CLI instructions
├── LICENSE              # MIT license
├── deny.toml            # cargo-deny v2 supply chain config
├── rust-toolchain.toml  # Pinned stable toolchain
└── rustfmt.toml         # Rustfmt 2024 edition settings
```

## CI Pipeline

The CI pipeline runs the following jobs on every push to `main`/`develop` and all PRs:

| Job | Tool | Purpose |
|-----|------|---------|
| Format | `cargo fmt` | Code formatting check |
| Clippy | `cargo clippy` | Lint with all targets, all features, -D warnings |
| Test | `cargo nextest` | Run tests with CI profile |
| Security Audit | `cargo audit` | Check for known vulnerabilities |
| Dependency Policy | `cargo deny` | License and supply chain checks |
| MSRV Check | `cargo check` | Verify MSRV 1.87 compatibility |
| CI Success | Gate | All jobs must pass |

## AI Agent Integration

This template includes instruction files for popular AI coding assistants:

- **`AGENTS.md`** - For OpenAI Codex and compatible agents
- **`CLAUDE.md`** - For Claude Code (Anthropic)
- **`GEMINI.md`** - For Gemini CLI (Google)
- **`.agents/skills/`** - Reusable agent skill files for common tasks

## Local Quality Gates

Run all quality checks locally before pushing:

```bash
bash scripts/quality-gates.sh
```

## License

MIT - see [LICENSE](LICENSE)
