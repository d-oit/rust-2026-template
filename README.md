# rust-2026-template

[![CI](https://github.com/d-oit/rust-2026-template/actions/workflows/ci.yml/badge.svg)](https://github.com/d-oit/rust-2026-template/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust 1.87+](https://img.shields.io/badge/rust-1.87%2B-orange.svg)](https://www.rust-lang.org)

Best practice 2026 Rust GitHub repository template with AI agent support, CI/CD, quality gates, and modern Rust tooling.

## Features

- **Workspace structure** — Multi-crate Cargo workspace with `crates/` layout; includes `example-crate` (library) and `sample-app` (binary)
- **Rust 2024 edition** — MSRV 1.87, pinned via `rust-toolchain.toml`
- **CI/CD pipeline** — GitHub Actions: format, clippy, nextest, security audit, cargo-deny, MSRV check, release
- **AI agent support** — `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, `QWEN.md`, `opencode.json`
- **Agent skills** — `.agents/skills/` with 9 reusable skill files
- **Quality gates** — `scripts/quality-gates.sh` (9 checks) and `scripts/code-quality.sh`
- **Supply chain security** — `deny.toml` with cargo-deny v2; `cargo-audit` in CI
- **Modern linting** — `.clippy.toml` with pedantic lints; zero-warnings policy
- **Formatting** — `rustfmt.toml` with Rust 2024 edition settings
- **WSL2/Linux optimizations** — mold linker, disk-space-optimized dev profile in `.cargo/config.toml`
- **cargo-nextest** — `.config/nextest.toml` with `default` and `ci` profiles
- **Architecture docs** — `plans/adr/`, `docs/architecture/context.yaml`, `agents-docs/`

## Quick Start

See **[QUICKSTART.md](QUICKSTART.md)** for the full setup guide.

1. Click **"Use this template"** on GitHub
2. Check crates.io availability: `cargo search your-crate-name`
3. Rename `crates/example-crate` to your crate name
4. Update `Cargo.toml` workspace metadata
5. Update agent instruction files with your project details
6. Push to `main` — CI runs automatically

## Workspace Crates

| Crate | Type | Description |
|---|---|---|
| [`example-crate`](crates/example-crate/) | Library | Minimal library placeholder — rename and replace |
| [`sample-app`](crates/sample-app/) | Binary | Full-featured app demonstrating tokio, clap, serde, tracing, thiserror |

## Agent Skills

All skills live in `.agents/skills/` and are compatible with Claude Code, OpenCode, Gemini CLI, and Qwen Code.

| Skill | Purpose |
|---|---|
| [`build-rust`](.agents/skills/build-rust/) | Build Rust projects correctly |
| [`lint-rust`](.agents/skills/lint-rust/) | Run clippy and formatting checks |
| [`test-rust`](.agents/skills/test-rust/) | Run tests with cargo-nextest |
| [`release-rust`](.agents/skills/release-rust/) | Safe release workflow for crates.io |
| [`crates-io-name-check`](.agents/skills/crates-io-name-check/) | Verify crate name availability before publishing |
| [`anti-ai-slop`](.agents/skills/anti-ai-slop/) | Audit and fix generic AI-generated Rust code patterns |
| [`privacy-first`](.agents/skills/privacy-first/) | Prevent email/personal data from entering the codebase |
| [`skill-creator`](.agents/skills/skill-creator/) | Create and optimize new agent skills |
| [`skill-evaluator`](.agents/skills/skill-evaluator/) | Evaluate skill quality with structure checks |

## AI Agent Integration

| File | Agent |
|---|---|
| `AGENTS.md` | OpenAI Codex and compatible agents |
| `CLAUDE.md` | Claude Code (Anthropic) |
| `GEMINI.md` | Gemini CLI (Google) |
| `QWEN.md` | Qwen Code |
| `opencode.json` | OpenCode configuration |
| `.agents/skills/` | Reusable skill runbooks |

## Repository Structure

```
rust-2026-template/
├── .agents/
│   ├── SKILLS.md            # Skills index
│   └── skills/              # 9 AI agent skill files
├── .cargo/
│   └── config.toml          # Linker, dev profile, cargo aliases
├── .config/
│   └── nextest.toml         # nextest profiles (default, ci)
├── .github/
│   ├── ISSUE_TEMPLATE/      # Bug report + feature request templates
│   ├── PULL_REQUEST_TEMPLATE.md
│   ├── dependabot.yml
│   └── workflows/
│       ├── ci.yml           # Format, clippy, test, audit, deny, MSRV
│       └── release.yml      # Tag-triggered release with cargo-dist
├── .vscode/
│   └── settings.json        # rust-analyzer + WSL2 settings
├── agents-docs/             # Agent reference docs
│   ├── commands.md
│   ├── conventions.md
│   ├── structure.md
│   └── workflow.md
├── crates/
│   ├── example-crate/       # Library placeholder
│   └── sample-app/          # Binary: tokio, clap, serde, tracing
├── docs/
│   └── architecture/
│       └── context.yaml
├── plans/
│   ├── GOAP_STATE.md        # AI agent world state
│   └── adr/                 # Architecture Decision Records
├── scripts/
│   ├── code-quality.sh      # fmt | clippy | audit | check | fix
│   ├── quality-gates.sh     # 9-step local quality gate runner
│   └── release-manager.sh   # validate | prepare | publish
├── src/
│   └── lib.rs               # Workspace-level lib template stub
├── .clippy.toml             # Clippy lint configuration
├── .gitignore
├── AGENTS.md                # AI agent instructions
├── CHANGELOG.md
├── CLAUDE.md
├── CONTRIBUTING.md
├── Cargo.toml               # Workspace manifest
├── GEMINI.md
├── LICENSE                  # MIT
├── MIGRATION.md
├── QUICKSTART.md
├── QWEN.md
├── SECURITY.md
├── deny.toml                # cargo-deny v2 supply chain config
├── opencode.json
├── rust-toolchain.toml      # Pinned stable 1.87
└── rustfmt.toml             # Rustfmt 2024 edition settings
```

## CI Pipeline

Runs on every push to `main`/`develop` and all PRs:

| Job | Tool | Purpose |
|---|---|---|
| Format | `cargo fmt` | Formatting check |
| Clippy | `cargo clippy` | All targets, all features, `-D warnings` |
| Test | `cargo nextest` | Tests + doc tests, CI profile |
| Security Audit | `cargo audit` | Known vulnerability check |
| Dependency Policy | `cargo deny` | License and supply chain checks |
| MSRV Check | `cargo check` | Verify MSRV 1.87 compatibility |
| CI Success | Gate | All jobs must pass before merge |

## Local Quality Gates

```bash
# Run all 9 checks (mirrors CI)
bash scripts/quality-gates.sh

# Auto-fix formatting and clippy issues
bash scripts/quality-gates.sh --fix

# Individual operations
./scripts/code-quality.sh fmt      # format
./scripts/code-quality.sh clippy   # lint
./scripts/code-quality.sh audit    # security audit
./scripts/code-quality.sh check    # full CI parity
```

## Cargo Aliases

Defined in `.cargo/config.toml`:

```bash
cargo check-all     # cargo check --workspace --all-features
cargo test-all      # cargo nextest run --workspace
cargo fmt-check     # cargo fmt --all -- --check
cargo lint          # cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo audit-check   # cargo deny check
```

## Documentation

- **[QUICKSTART.md](QUICKSTART.md)** — Setup guide
- **[CONTRIBUTING.md](CONTRIBUTING.md)** — Contribution process
- **[SECURITY.md](SECURITY.md)** — Security policy and vulnerability reporting
- **[MIGRATION.md](MIGRATION.md)** — Adopt template in existing projects / upgrade between versions
- **[CHANGELOG.md](CHANGELOG.md)** — Version history

## License

MIT — see [LICENSE](LICENSE)
