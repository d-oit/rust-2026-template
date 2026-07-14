# Project Structure

```
rust-2026-template/
├── .agents/
│   ├── SKILLS.md                  # Auto-generated skills index
│   ├── ORCHESTRATION.md           # Multi-agent orchestration rules
│   ├── ci/                        # CI health status artifacts
│   │   └── ci-summary.md
│   ├── context/                   # Cross-repo context for derived repositories
│   │   ├── external-repos.json
│   │   ├── shared-conventions.md
│   │   └── workflow-state.json
│   ├── events/                    # Agentic metrics event files (DORA)
│   └── skills/                    # AI agent skill runbooks
│       ├── anti-ai-slop/
│       ├── architecture-diagram/
│       ├── atomic-commit/
│       ├── build-rust/
│       ├── codacy/
│       ├── crates-io-name-check/
│       ├── dora-report/
│       ├── goap-agent/
│       ├── harness/
│       ├── issue-triage/
│       ├── lint-rust/
│       ├── metrics-reporter/
│       ├── privacy-first/
│       ├── release-rust/
│       ├── secret-lint/
│       ├── self-fix-loop/
│       ├── skill-creator/
│       ├── skill-evaluator/
│       ├── task-decomposition/
│       ├── test-rust/
│       ├── triz-analysis/
│       ├── triz-solver/
│       └── verify-actions/
├── .cargo/
│   └── config.toml                # Linker (mold), dev profile, cargo aliases
├── .claude/                       # Claude Code integration
├── .config/
│   └── nextest.toml               # nextest profiles: default, ci
├── .gemini/                       # Gemini CLI integration
├── .github/
│   ├── ISSUE_TEMPLATE/            # bug_report.yml, feature_request.yml
│   ├── PULL_REQUEST_TEMPLATE.md
│   ├── actions/                   # Reusable composite actions
│   ├── dependabot.yml
│   ├── release-drafter.yml
│   └── workflows/                 # GitHub Actions workflows
│       ├── ci.yml                 # Format, clippy, test, audit, deny, MSRV
│       ├── release.yml            # Tag-triggered release with cargo-dist
│       ├── commitlint.yml
│       ├── dependabot-auto-merge.yml
│       ├── deploy-docs.yml
│       ├── docs-check.yml
│       ├── dora-fdrt.yml
│       ├── dora-report.yml
│       ├── eval.yml
│       ├── fuzz.yml
│       ├── hotfix.yml
│       ├── labeler.yml
│       ├── mutants.yml
│       ├── secretlint.yml
│       ├── security-scan.yml
│       └── ...                    # +8 more workflows
├── .opencode/                     # OpenCode integration
├── .qwen/                         # Qwen Code integration
├── .windsurf/                     # Windsurf integration
├── .vscode/
│   └── settings.json              # rust-analyzer + WSL2 settings
├── agents-docs/                   # Agent reference documentation
│   ├── agent-doc-flow.md
│   ├── commands.md                # Command quick reference
│   ├── conventions.md             # Code conventions and invariants
│   ├── dora-metrics.md
│   ├── HANDOFF.md
│   ├── HOOKS.md
│   ├── LESSONS.md
│   ├── structure.md               # This file
│   ├── SUB-AGENTS.md
│   └── workflow.md                # Change workflow and skill/CLI pattern
├── benchmarks/                    # Criterion benchmark suites
│   └── Cargo.toml
├── config/
│   └── profiles/                  # Environment-specific JSON configs
├── crates/                        # Workspace member crates
│   ├── actor-runtime-template/    # Message-passing concurrency pattern
│   ├── checkpoint-template/       # Serializable state + migrations
│   ├── example-crate/             # Placeholder library (rename for your project)
│   ├── example-registry-pattern/  # Handler map by name
│   ├── example-storage-pattern/   # Trait-only storage pattern
│   ├── hybrid-storage-template/   # Multi-backend storage wrapper
│   ├── mcp-server-template/       # MCP tool registry pattern
│   ├── sample-app/                # Reference binary application
│   └── xtask/                     # Cargo task runner
├── docs/
│   ├── adr/                       # Architecture Decision Records
│   ├── architecture/              # Architecture context for AI agents
│   ├── patterns/                  # Template pattern guides
│   └── src/                       # mdbook source files
├── examples/
│   └── hello_world/               # Simple hello world example
├── fuzz/                          # cargo-fuzz testing scaffold
│   └── Cargo.toml
├── hooks/                         # Git hooks (session-start.sh, etc.)
├── monitoring/                    # Monitoring configuration
├── plans/
│   ├── GOAP_STATE.md              # AI agent world state tracker
│   └── adr/                       # Architecture Decision Records
├── reports/                       # Generated HTML reports (git-ignored)
├── schema/                        # JSON Schema definitions
├── scripts/                       # Automation scripts
│   ├── bootstrap.sh               # One-command first-time setup
│   ├── code-quality.sh            # fmt | clippy | audit | check | fix
│   ├── doctor.sh                  # Environment diagnostics
│   ├── generate-llms-txt.sh       # Regenerate LLM context files
│   ├── generate-skills-md.sh      # Regenerate skills index
│   ├── init-template.sh           # Initialize new project from template
│   ├── quality-gates.sh           # Local quality gate runner
│   ├── release-manager.sh         # validate | prepare | publish
│   ├── validate-skills.sh         # Skill validation
│   └── ...                        # +21 more scripts
├── templates/                     # Template files
├── tests/                         # Integration tests
├── .clippy.toml                   # Clippy lint configuration
├── .envrc                         # direnv environment setup
├── .gitignore
├── AGENTS.md                      # Canonical AI agent instructions
├── CHANGELOG.md                   # Keep-a-changelog format
├── CLAUDE.md                      # Claude Code adapter
├── CONTRIBUTING.md
├── Cargo.lock
├── Cargo.toml                     # Workspace manifest
├── GEMINI.md                      # Gemini CLI adapter
├── HARNESS.md                     # Harness engineering guide
├── LICENSE                        # MIT
├── MIGRATION.md                   # Template adoption guide
├── QUICKSTART.md                  # Setup and onboarding guide
├── QWEN.md                        # Qwen Code adapter
├── SECURITY.md
├── VERSION                        # Plain-text version (0.0.0 for adopter)
├── deny.toml                      # cargo-deny v2 supply chain config
├── docflow.json                   # Agent session bootstrap config
├── flake.nix                      # Nix flake (optional)
├── llms.txt                       # LLM context file (machine-readable)
├── llms-full.txt                  # Full LLM source context (auto-generated)
├── opencode.json                  # OpenCode configuration
├── rust-toolchain.toml            # Pinned stable 1.88
└── rustfmt.toml                   # Rustfmt 2024 edition settings
```
