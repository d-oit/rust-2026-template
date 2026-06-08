# Contributing to rust-2026-template

Thank you for considering contributing! This is a generic Rust project template.

## Quick Links

- [Issues](https://github.com/d-oit/rust-2026-template/issues)
- [Pull Requests](https://github.com/d-oit/rust-2026-template/pulls)
- [Security Policy](SECURITY.md)

## Development Setup

### Prerequisites

- Rust stable (see `rust-toolchain.toml` for exact version)
- `cargo-nextest` — `cargo install cargo-nextest`
- `cargo-deny` — `cargo install cargo-deny`
- `cargo-audit` — `cargo install cargo-audit`

### Clone and Build

```bash
git clone https://github.com/d-oit/rust-2026-template.git
cd rust-2026-template
cargo build
```

### Run Quality Gates Locally

Always run before pushing:

```bash
bash scripts/quality-gates.sh
```

This runs: `cargo fmt --check`, `cargo clippy`, `cargo nextest run`, `cargo audit`, `cargo deny check`.

## Making Changes

### Branch Naming

| Type | Pattern | Example |
|---|---|---|
| Feature | `feat/description` | `feat/add-async-support` |
| Bug fix | `fix/description` | `fix/clippy-warnings` |
| Docs | `docs/description` | `docs/update-readme` |
| Refactor | `refactor/description` | `refactor/workspace-layout` |

### Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(crate-name): add feature X
fix: resolve clippy warning in lib.rs
docs: update AGENTS.md with new skill
chore(deps): bump serde from 1.0.195 to 1.0.196
```

### Code Style

- Format: `cargo fmt` (enforced by CI)
- Lint: `cargo clippy -- -D warnings` (zero warnings policy)
- Shell Lint: `shellcheck` for all scripts in `scripts/` (enforced by CI)
- Edition: Rust 2024
- MSRV: 1.88 (see `rust-toolchain.toml`)

### Tests

- Use `cargo nextest run` for all tests
- Unit tests live in `#[cfg(test)]` modules in source files
- Integration tests live in `crates/<name>/tests/`
- All public items must have doc tests or unit tests

## Pull Request Process

1. Fork the repository
2. Create a feature branch: `git checkout -b feat/my-feature`
3. Make changes and run quality gates
4. Commit using Conventional Commits format
5. Push and open a PR against `main`
6. Wait for CI to pass (all green required)
7. Request review

### PR Checklist

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo nextest run` passes
- [ ] `cargo audit` shows no vulnerabilities
- [ ] `cargo deny check` passes
- [ ] Coverage targets met (as defined in `.codecov.yml`)
- [ ] Documentation updated if API changed
- [ ] `CHANGELOG.md` updated
- [ ] `shellcheck` passes for all shell scripts

## Release Process

We use `cargo-release` for version management and `cargo-dist` for artifact generation.

### Cutting a Release

1. Ensure you are on the `main` branch and it's up to date.
2. Run `cargo release <patch|minor|major>` to prepare the release.
    - This will run quality gates (via `scripts/pre-release-hook.sh`), bump versions, update the changelog (via `git-cliff`), and create a tag.
3. Push the tag to trigger the GitHub Actions release workflow.

```bash
cargo release patch --execute
git push --tags
```

## Template-Specific Guidance

This is a **generic Rust template**, not a standalone application. Changes should:

- Remain generic and reusable for any Rust project
- Not add application-specific logic
- Keep the `example-crate` as a minimal, illustrative placeholder
- Be documented in `CHANGELOG.md`

### Publishing to crates.io

Every publishable crate **must** define an `include` whitelist in its `Cargo.toml`
to prevent internal files (plans, agent docs, scripts, CI config) from ending up in
the published package. The template already sets this up via `[workspace.package]`:

```toml
include = ["/src", "README.md", "LICENSE"]
```

When you create a new crate, verify the publish surface is correct:

```bash
cargo package --list -p your-crate
```

The output should **not** contain `plans/`, `agents-docs/`, `scripts/`, `.github/`,
`.agents/`, or `.opencode/`. See
[Cargo manifest include field](https://doc.rust-lang.org/cargo/reference/manifest.html#the-include-and-exclude-fields)
for details.

## Reporting Issues

Open an issue at: <https://github.com/d-oit/rust-2026-template/issues>

For security vulnerabilities, see [SECURITY.md](SECURITY.md).
