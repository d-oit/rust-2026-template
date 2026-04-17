# Project Structure

```
rust-2026-template/
├── .agents/skills/      # AI agent skill definitions
├── .cargo/config.toml   # Cargo linker + profile config
├── .claude/             # Claude-specific config
├── .config/nextest.toml # nextest profiles
├── .github/
│   ├── workflows/       # CI/CD GitHub Actions
│   └── PULL_REQUEST_TEMPLATE.md
├── .vscode/settings.json # VS Code / WSL2 settings
├── scripts/             # Dev helper scripts
├── plans/adr/           # Architecture Decision Records
├── AGENTS.md            # AI agent instructions (this file)
├── CLAUDE.md            # Claude: @AGENTS.md
├── GEMINI.md            # Gemini: @AGENTS.md
└── Cargo.toml           # Workspace manifest
```
