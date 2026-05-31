# Project Structure

```
rust-2026-template/
├── .agents/
│   ├── SKILLS.md            # Skills index
│   └── skills/              # 9 AI agent skill runbooks
│       ├── anti-ai-slop/
│       ├── build-rust/
│       ├── crates-io-name-check/
│       ├── lint-rust/
│       ├── privacy-first/
│       ├── release-rust/
│       ├── skill-creator/
│       ├── skill-evaluator/
│       └── test-rust/
├── .cargo/
│   └── config.toml          # Linker (mold), dev profile, cargo aliases
├── .config/
│   └── nextest.toml         # nextest profiles: default, ci
├── .github/
│   ├── ISSUE_TEMPLATE/      # bug_report.yml, feature_request.yml
│   ├── PULL_REQUEST_TEMPLATE.md
│   ├── dependabot.yml
│   └── workflows/
│       ├── ci.yml           # Format, clippy, test, audit, deny, MSRV
│       └── release.yml      # Tag-triggered release with cargo-dist
├── .vscode/
│   └── settings.json        # rust-analyzer + WSL2 settings
├── agents-docs/             # Agent reference documentation
│   ├── commands.md          # Command quick reference
│   ├── conventions.md       # Code conventions and invariants
│   ├── structure.md         # This file
│   └── workflow.md          # Change workflow and skill/CLI pattern
├── crates/
│   ├── example-crate/       # Library placeholder (rename for your project)
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   └── src/lib.rs
│   └── sample-app/          # Binary: tokio, clap, serde, tracing, thiserror
│       ├── Cargo.toml
│       ├── README.md
│       └── src/main.rs
├── docs/
│   └── architecture/
│       └── context.yaml     # Architecture context for AI agents
├── plans/
│   ├── GOAP_STATE.md        # AI agent world state tracker
│   └── adr/
│       └── 0001-rust-edition-and-toolchain.md
├── scripts/
│   ├── code-quality.sh      # fmt | clippy | audit | check | fix
│   ├── quality-gates.sh     # 9-step local quality gate runner (--fix flag)
│   └── release-manager.sh   # validate | prepare | publish (dry-run by default)
├── src/
│   └── lib.rs               # Workspace-level lib stub (template placeholder)
├── .clippy.toml             # Clippy lint configuration
├── .envrc                   # direnv environment setup
├── .gitignore
├── AGENTS.md                # AI agent instructions
├── CHANGELOG.md             # Keep-a-changelog format
├── CLAUDE.md                # Claude Code: @AGENTS.md
├── CONTRIBUTING.md
├── Cargo.lock
├── Cargo.toml               # Workspace manifest
├── GEMINI.md                # Gemini CLI: @AGENTS.md
├── LICENSE                  # MIT
├── MIGRATION.md
├── QUICKSTART.md
├── QWEN.md                  # Qwen Code: @AGENTS.md
├── SECURITY.md
├── deny.toml                # cargo-deny v2 supply chain config
├── flake.nix                # Nix flake (optional)
├── opencode.json            # OpenCode configuration
├── rust-toolchain.toml      # Pinned stable 1.88
└── rustfmt.toml             # Rustfmt 2024 edition settings
```
