# Rust 2026 Template — mdbook

A production-ready Rust workspace template with modern tooling, CI/CD, and AI agent integration.

## Chapters

| Chapter | Description |
|---------|-------------|
| [Getting Started](./getting-started.md) | Prerequisites, setup, and first build |
| [Architecture](./architecture.md) | Workspace layout, crate layering, and design decisions |
| [CI/CD](./ci.md) | GitHub Actions workflows, quality gates, and local parity |
| [DORA Metrics](./dora-metrics.md) | Measuring delivery performance with agentic metrics |
| [Faster Builds](./faster-builds.md) | Compilation speed optimizations per platform |

## Quick Start

```bash
./scripts/bootstrap.sh
cargo build --workspace
cargo nextest run --workspace
```

## Key Files

- **AGENTS.md** — Canonical instructions for AI coding agents
- **QUICKSTART.md** — Human onboarding and setup guide
- **.agents/skills/** — Reusable skill runbooks for agents
- **scripts/quality-gates.sh** — Local pre-push checks
