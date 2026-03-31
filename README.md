# rust-2026-template

[![CI](https://github.com/d-oit/rust-2026-template/actions/workflows/ci.yml/badge.svg)](https://github.com/d-oit/rust-2026-template/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust 1.87+](https://img.shields.io/badge/rust-1.87%2B-orange.svg)](https://www.rust-lang.org)

Best practice 2026 Rust GitHub repository template with AI agent support, CI/CD, quality gates, and all modern Rust tooling.

## Features

- **Workspace structure** - Multi-crate Cargo workspace with `crates/` layout
- **2026 Rust edition** - Rust edition 2024, MSRV 1.87
- **CI/CD pipeline** - Full GitHub Actions CI with format, clippy, test, security audit, cargo-deny, MSRV check
- **AI agent support** - `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, `QWEN.md`, `opencode.json` for AI coding assistants
- **Agent skills** - `.agents/skills/` directory with 9 reusable agent skill files
- **Quality gates** - `scripts/quality-gates.sh` for local quality checks
- **Architecture decisions** - `plans/adr/` with ADR template
- **Supply chain security** - `deny.toml` with cargo-deny v2 configuration
- **Modern linting** - `.clippy.toml` with pedantic lints
- **Formatting** - `rustfmt.toml` with Rust 2024 edition settings
- **VSCode** - `.vscode/settings.json` with rust-analyzer and WSL2 support
- **cargo-nextest** - `.config/nextest.toml` with CI profile
- **Documentation** - `QUICKSTART.md`, `CONTRIBUTING.md`, `SECURITY.md`, `MIGRATION.md`

## Quick Start

See **[QUICKSTART.md](QUICKSTART.md)** for the full 5-minute setup guide.

1. Click **"Use this template"** on GitHub to create a new repo
2. **Check crates.io** for your crate name: `cargo search your-crate-name`
3. Rename `example-crate` under `crates/` with your own crate(s)
4. Update `Cargo.toml` workspace metadata (name, authors, repository, etc.)
5. Update `AGENTS.md`, `CLAUDE.md`, `GEMINI.md` with your project details
6. Push to `main` and watch CI pass!

## Agent Skills

All skills live in `.agents/skills/` and are compatible with Claude Code, OpenCode, Gemini CLI, and Qwen Code.

| Skill | Purpose |
|---|---|
| [`build-rust`](.agents/skills/build-rust/) | Build Rust projects correctly |
| [`lint-rust`](.agents/skills/lint-rust/) | Run clippy and formatting checks |
| [`test-rust`](.agents/skills/test-rust/) | Run tests with cargo-nextest |
| [`release-rust`](.agents/skills/release-rust/) | Safe release workflow for crates.io |
| [`crates-io-name-check`](.agents/skills/crates-io-name-check/) | Verify crate name is unique before publishing |
| [`anti-ai-slop`](.agents/skills/anti-ai-slop/) | Audit/fix generic AI-generated Rust code patterns |
| [`privacy-first`](.agents/skills/privacy-first/) | Prevent email/personal data from entering codebase |
| [`skill-creator`](.agents/skills/skill-creator/) | Create and optimize new agent skills |
| [`skill-evaluator`](.agents/skills/skill-evaluator/) | Evaluate skill quality with structure checks and evals |

## AI Agent Integration

This template includes instruction files for popular AI coding assistants:

- **`AGENTS.md`** - For OpenAI Codex and compatible agents
- **`CLAUDE.md`** - For Claude Code (Anthropic)
- **`GEMINI.md`** - For Gemini CLI (Google)
- **`QWEN.md`** - For Qwen Code
- **`opencode.json`** - OpenCode configuration
- **`.agents/skills/`** - Reusable agent skill files for common tasks

## Repository Structure

```
rust-2026-template/
├── .agents/
│   └── skills/          # AI agent skill files (9 skills)
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
├── plans/
│   └── adr/             # Architecture Decision Records
├── scripts/
│   └── quality-gates.sh # Local quality gate runner
├── .clippy.toml         # Clippy lint configuration
├── .gitignore           # Rust + WSL2 + IDE gitignore
├── AGENTS.md            # AI agent instructions
├── CHANGELOG.md         # Keep-a-changelog format
├── CLAUDE.md            # Claude Code instructions
├── CONTRIBUTING.md      # Contribution guidelines
├── Cargo.toml           # Workspace Cargo manifest
├── GEMINI.md            # Gemini CLI instructions
├── LICENSE              # MIT license
├── MIGRATION.md         # Template adoption & upgrade guide
├── QUICKSTART.md        # 5-minute setup guide
├── QWEN.md              # Qwen Code instructions
├── SECURITY.md          # Security policy
├── deny.toml            # cargo-deny v2 supply chain config
├── opencode.json        # OpenCode configuration
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

## Local Quality Gates

Run all quality checks locally before pushing:

```bash
bash scripts/quality-gates.sh
```

## Documentation

- 🚀 **[QUICKSTART.md](QUICKSTART.md)** - 5-minute setup guide
- 🔧 **[CONTRIBUTING.md](CONTRIBUTING.md)** - How to contribute
- 🔒 **[SECURITY.md](SECURITY.md)** - Security policy and vulnerability reporting
- 🔄 **[MIGRATION.md](MIGRATION.md)** - Adopt template in existing projects / upgrade between versions
- 📝 **[CHANGELOG.md](CHANGELOG.md)** - Version history

## License

MIT - see [LICENSE](LICENSE)
