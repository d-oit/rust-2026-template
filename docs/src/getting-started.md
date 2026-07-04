# Getting Started

## Prerequisites

- **Rust** 1.88+ via [rustup](https://rustup.rs)
- **Git** 2.30+
- **Python 3** (optional, for TOML/YAML validation)

## Quick Setup

Clone and bootstrap in one command:

```bash
git clone https://github.com/your-org/your-repo.git
cd your-repo
./scripts/bootstrap.sh
```

The bootstrap script:
1. Checks your environment (git, cargo)
2. Installs skill symlinks for AI agents
3. Configures git hooks for pre-commit quality checks
4. Validates all skills
5. Runs the full quality gate

## Manual Setup

If you prefer step-by-step:

```bash
# 1. Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Build the workspace
cargo build --workspace

# 3. Run tests
cargo nextest run --workspace

# 4. Run quality checks
./scripts/quality-gates.sh
```

## Project Structure

```
.
├── crates/                  # Workspace members
│   ├── sample-app/          # Reference binary application
│   ├── example-crate/       # Placeholder library
│   ├── actor-runtime-template/
│   ├── checkpoint-template/
│   ├── hybrid-storage-template/
│   ├── mcp-server-template/
│   └── example-*/
├── examples/                # Example binaries
├── benchmarks/              # Criterion benchmarks
├── scripts/                 # Automation scripts
├── .agents/skills/          # AI agent skills
└── docs/                    # Documentation (mdbook)
```

## Common Commands

| Task | Command |
|------|---------|
| Build | `cargo build --workspace` |
| Test | `cargo nextest run --workspace` |
| Lint | `cargo clippy --workspace --all-targets` |
| Format | `cargo fmt --all` |
| Quality gate | `./scripts/quality-gates.sh` |
| Diagnostics | `./scripts/doctor.sh` |

## Feature Flags

Enable optional features in `Cargo.toml`:

```toml
[dependencies]
rust-2026-template = { path = "..", features = ["cli", "persistence"] }
```

| Feature | Description |
|---------|-------------|
| `cli` | CLI binary support (clap, anyhow) |
| `persistence` | SQL persistence backend (libsql) |
| `parallel` | CPU parallelism (rayon) |
| `wasm` | WASM build target support |
| `tracing-json` | JSON tracing output |
| `tracing-opentelemetry` | OpenTelemetry backend |

## IDE Setup

**VS Code** — Install the `rust-analyzer` extension. The workspace is pre-configured with optimal settings in `.cargo/config.toml`.

**Other editors** — Ensure `rust-analyzer` or `rLS` is installed. The `rust-toolchain.toml` file auto-selects the correct toolchain.

## Troubleshooting

Run the diagnostic script:

```bash
./scripts/doctor.sh
```

This checks:
- Rust toolchain version
- Required tools (cargo-nextest, cargo-deny, etc.)
- Git configuration
- Skill symlinks
- Build health
