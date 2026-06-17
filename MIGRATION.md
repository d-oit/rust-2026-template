# Migration Guide

This document helps you adopt `rust-2026-template` in existing projects, or upgrade
between template versions.

## Adopting the Template in an Existing Rust Project

### Step 1: Copy Configuration Files

From the template root, copy these files to your project:

```bash
# Rust toolchain and formatting
cp rust-toolchain.toml your-project/
cp rustfmt.toml your-project/
cp .clippy.toml your-project/

# Supply chain security
cp deny.toml your-project/

# cargo-nextest config
mkdir -p your-project/.config
cp .config/nextest.toml your-project/.config/

# Quality gate script
mkdir -p your-project/scripts
cp scripts/quality-gates.sh your-project/scripts/
chmod +x your-project/scripts/quality-gates.sh

# Release engineering
cp dist-workspace.toml your-project/
cp release.toml your-project/
cp cliff.toml your-project/
cp scripts/pre-release-hook.sh your-project/scripts/
chmod +x your-project/scripts/pre-release-hook.sh
```

### Step 2: Copy Agent Files

```bash
cp AGENTS.md your-project/
cp CLAUDE.md your-project/
cp GEMINI.md your-project/
cp QWEN.md your-project/
cp opencode.json your-project/

# Copy skills
mkdir -p your-project/.agents/skills
cp -r .agents/skills/* your-project/.agents/skills/
```

Update `AGENTS.md` with your project name and description.

### Step 3: Copy CI/CD Workflows

```bash
mkdir -p your-project/.github/workflows
cp .github/workflows/ci.yml your-project/.github/workflows/
cp .github/workflows/release.yml your-project/.github/workflows/
cp .github/dependabot.yml your-project/.github/
```

Update the workflow files to match your crate names.

### Step 4: Update Cargo.toml

Ensure your workspace `Cargo.toml` has at minimum:

```toml
[workspace]
resolver = "3"

[workspace.package]
edition = "2024"
rust-version = "1.88"
```

### Step 5: Update .gitignore

Merge the template `.gitignore` with your existing one.
Key additions: `/target`, `*.swp`, `.direnv/`, `.envrc`.

---

## Template Version Upgrades

### Upgrading from v0.x to v0.2+

| Change | Action Required |
|---|---|
| `plans/adr/` replaces `docs/adr/` | Move ADR files: `mv docs/adr plans/adr` |
| `agents-docs/` added | Copy new reference docs from template |
| `.agents/skills/crates-io-name-check/` added | Copy skill directory |
| `opencode.json` added | Copy file to repo root |
| `CONTRIBUTING.md`, `SECURITY.md` added | Copy and customize |
| New skills: `anti-ai-slop`, `privacy-first`, `skill-creator`, `skill-evaluator` | Copy skill directories |

---

## Crate Naming (Template Rename)

When using this template, always rename `example-crate` before first publish:

1. Rename the directory: `mv crates/example-crate crates/your-crate-name`
2. Update `crates/your-crate-name/Cargo.toml`: `name = "your-crate-name"`
3. Update root `Cargo.toml` workspace members if needed
4. Check crates.io: `cargo search your-crate-name`
5. See `.agents/skills/crates-io-name-check/SKILL.md` for full name-check workflow

---

## Breaking Changes by Version

### v0.2.0

- `plans/adr/` is now the canonical ADR location (was `docs/adr/`)
- Rust toolchain pinned to `1.87` (was `1.85`)
- `cargo-deny` v2 syntax in `deny.toml`
- New required skills: `crates-io-name-check` in release workflow
- **Release profile updated:** `lto` changed from `"thin"` to `"fat"` for maximum
  runtime performance. `panic = "unwind"` and `strip = "symbols"` are now explicit.
  If you had overridden the release profile, review your settings against the new
  defaults. Use `lto = "thin"` during development for faster builds.

### v0.1.0

- Initial template release
