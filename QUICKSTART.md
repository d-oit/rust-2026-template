# Quick Start — rust-2026-template

Get a new Rust project running in under 5 minutes.

## Prerequisites

- Rust stable via [rustup](https://rustup.rs/) — toolchain pinned to 1.88 in `rust-toolchain.toml`
- Git 2.30+
- Optional: `cargo-nextest`, `cargo-deny`, `cargo-audit` for full quality gates

## 1. Create Your Project from the Template

1. Click **"Use this template"** on GitHub
2. Name your repository and create it
3. Clone it locally:

```bash
git clone https://github.com/YOUR_USER/YOUR_REPO.git
cd YOUR_REPO
```

## 2. Rename the Example Crate

**Before anything else**, rename `example-crate` to your crate name:

```bash
# Check the name is available on crates.io first
cargo search your-crate-name
# If no exact match: available!

# Rename directory and update Cargo.toml
mv crates/example-crate crates/your-crate-name
# Edit crates/your-crate-name/Cargo.toml
# Change: name = "example-crate" -> name = "your-crate-name"
```

See `.agents/skills/crates-io-name-check/SKILL.md` for the full name-check workflow.

The `sample-app` binary crate can be kept as a reference or renamed/removed as needed.

## 3. Install Required Tools

```bash
# Required for tests
cargo install cargo-nextest
cargo install cargo-llvm-cov

# Required for CI (supply chain checks)
cargo install cargo-deny
cargo install cargo-audit
```

## 4. Build and Test

```bash
# Build all workspace crates
cargo build --workspace

# Run all tests
cargo nextest run --workspace

# Run the sample app
cargo run -p sample-app
cargo run -p sample-app -- --count 5 --verbose
```

## 5. Run All Quality Gates

```bash
bash scripts/quality-gates.sh
```

Runs 9 checks: format, clippy, build, tests, doc tests, security audit, cargo-deny, unused deps, privacy scan, secret scan.

Pass `--fix` to auto-correct formatting and clippy issues:

```bash
bash scripts/quality-gates.sh --fix
```

## 6. Update Project Metadata

Edit these files with your project details:

| File | What to update |
|---|---|
| `Cargo.toml` | `authors`, `repository`, `homepage`, `documentation` |
| `crates/*/Cargo.toml` | `name`, `description` |
| `AGENTS.md` | Project name, description, domain context |
| `CLAUDE.md` | Project-specific overrides |
| `config/profiles/default.json` | Application configuration defaults |
| `schema/config.schema.json` | JSON Schema for configuration validation |
| `README.md` | Replace template content with your project |
| `CODECOV_TOKEN` | Add to GitHub Actions secrets for coverage reporting |
| `SECURITY.md` | Your security contact / advisory link |
| `CONTRIBUTING.md` | Your contribution process |

### 7. Template Cleanup (Optional)

This template includes internal maintenance files that are not needed for your project. You can safely remove them:

```bash
rm CHANGELOG-TEMPLATE.md
```

## 8. Push and Watch CI Pass

```bash
git add -A
git commit -m "feat: initialize project from rust-2026-template"
git push origin main
```

CI runs: format check, clippy, nextest, doc tests, security audit, cargo-deny, MSRV check.

---

## What You Get

| Component | Location | Purpose |
|---|---|---|
| CI pipeline | `.github/workflows/ci.yml` | Format, lint, test, audit on every push |
| Release workflow | `.github/workflows/release.yml` | Tag-triggered release with cargo-dist |
| Agent skills | `.agents/skills/` | 9 AI coding assistant skill runbooks |
| Quality gate script | `scripts/quality-gates.sh` | 9-step local pre-push checks |
| Code quality script | `scripts/code-quality.sh` | fmt \| clippy \| audit \| check \| fix |
| Release manager | `scripts/release-manager.sh` | validate \| prepare \| publish |
| ADR template | `plans/adr/` | Architecture decision records |
| Clippy config | `.clippy.toml` | Pedantic lint rules |
| Deny config | `deny.toml` | License and vulnerability policy |
| Nextest config | `.config/nextest.toml` | Test profiles (default + ci) |
| Codecov config | `.codecov.yml` | Coverage gate enforcement targets |
| Cargo aliases | `.cargo/config.toml` | `check-all`, `test-all`, `lint`, etc. |

## Advanced Testing

### Fuzz Testing

A fuzz testing scaffold is included using `cargo-fuzz`. This is particularly useful for testing parsers and complex logic against randomized input.

```bash
# Install cargo-fuzz (nightly required)
cargo install cargo-fuzz

# Run a specific fuzz target
cargo fuzz run fuzz_parse_input -- -max_total_time=30
```

The fuzzer is also configured to run weekly via GitHub Actions.

## Next Steps

- Read `AGENTS.md` to understand how AI coding assistants are configured
- Read `CONTRIBUTING.md` before making changes
- Check `MIGRATION.md` if adopting this template in an existing project
- See `agents-docs/conventions.md` for coding conventions enforced by agents
