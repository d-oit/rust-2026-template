# Quick Start — rust-2026-template

Get a new Rust project running in under 5 minutes.

> **⚠️ First-Time Setup Required**
> Before building or publishing, run `init-template.sh` to replace placeholder values
> (`Your Name`, `your-org/your-repo`, `your-crate`) with your project metadata.
> CI will show a warning if placeholders are still present (template repo intentionally has them).

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

### Recommended: minimal init (most apps)

```bash
./scripts/init-template.sh --minimal \
  --name your-crate-name \
  --description "Your description" \
  --author "Your Name" \
  --repo YOUR_USER/YOUR_REPO
```

`--minimal` keeps `sample-app` + your renamed lib crate + `xtask`, and removes
optional pattern crates (MCP, actor, storage demos, …) plus optional workflows
(DORA, mutants, eval, …). Core CI/security workflows stay.

**Versions:** leave `VERSION` / workspace version at `0.0.0` until you ship.
Template release notes (`v0.3.x`) are only in `.template/CHANGELOG-TEMPLATE.md`.

## 2. Rename the Example Crate (manual alternative)

If you skip `init-template.sh`, rename `example-crate` yourself:

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
Pattern crates: see [docs/patterns/README.md](docs/patterns/README.md).

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

# Use fast-dev profile for faster local iterations
# (Disables debug symbols and optimizes build scripts)
cargo build --profile fast-dev
cargo nextest run --profile fast-dev
```

## 5. Run All Quality Gates

```bash
bash scripts/quality-gates.sh
```

Runs 9 checks (including pedantic and nursery clippy lints by default): format, clippy, build, tests, doc tests, security audit, cargo-deny, unused deps, privacy scan, secret scan.

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

## 7. Push and Watch CI Pass

```bash
git add -A
git commit -m "feat: initialize project from rust-2026-template"
git push origin main
```

CI runs: format check, clippy, nextest, doc tests, security audit, cargo-deny, MSRV check.

## Cutting a Release

1. Ensure your working tree is clean.
2. Run: `cargo release --workspace patch` (or `minor` / `major`)
   - This bumps all workspace member versions together.
   - Tags the commit as `v<version>`.
   - Pushes the tag, triggering the release CI workflow.
3. The GitHub Actions release workflow will:
   - Run `git-cliff` to update `CHANGELOG.md`
   - Create a GitHub Release with the generated notes

For a dry-run (no changes): `cargo release --workspace patch --dry-run`

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
| Clippy config | `.clippy.toml` | Pedantic and nursery lint rules |
| Deny config | `deny.toml` | License and vulnerability policy |
| Nextest config | `.config/nextest.toml` | Test profiles (default + ci) |
| Codecov config | `.codecov.yml` | Coverage gate enforcement targets |
| Cargo aliases | `.cargo/config.toml` | `check-all`, `test-all`, `lint`, etc. |

## Advanced Testing

### Mutation Testing

The template includes `cargo-mutants` for verifying that your tests actually catch bugs. Mutation testing injects small code changes (mutants) and checks if your test suite detects them.

```bash
# Install cargo-mutants
cargo install cargo-mutants

# Run mutation tests (takes several minutes)
cargo mutants --workspace

# Run against a specific file
cargo mutants --file src/lib.rs

# Filter by pattern
cargo mutants -m "if.*None"
```

**Understanding results:**
- **Caught** — Your tests detected the mutation (good!)
- **Survived** — The mutation wasn't detected (tests need improvement)
- **Timeout** — Mutant took too long to test (usually not a concern)

CI runs mutation testing weekly and on pushes to `main`. Check `.github/workflows/mutants.yml` for the schedule.

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
- Read [Faster Builds](docs/src/faster-builds.md) to optimize your development workflow
- Check `MIGRATION.md` if adopting this template in an existing project
- See `agents-docs/conventions.md` for coding conventions enforced by agents

## Cross-Repo Context

If you're using this template across multiple repositories, the `.agents/context/` directory enables cross-repo agent context sharing:

| File | Purpose |
|------|---------|
| `.agents/context/external-repos.json` | Links to related repos and their agent context URLs |
| `.agents/context/shared-conventions.md` | Conventions that apply across all derived repos |

**Configuration for your org:**

1. Edit `.agents/context/external-repos.json` to add your related repositories
2. Update `.agents/context/shared-conventions.md` with org-wide rules
3. Agents in derived repos will automatically discover and apply these conventions

**Merge precedence** (when instructions conflict):
1. Local repo instructions (AGENTS.md, .agents/skills/) — highest
2. Imported context (.agents/context/) — secondary
3. Template defaults (upstream rust-2026-template) — fallback only
