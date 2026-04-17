---
name: crates-io-name-check
description: >
  Verify that a new Rust crate name is available and appropriate on crates.io.
  Use when creating a new crate, renaming a crate, or before first publish.
  Triggers on "check crate name", "is this name taken", "verify availability",
  or "naming best practices".
license: MIT
metadata:
  author: d-oit
  version: "1.0"
  source: d-o-hub/github-template-ai-agents
  tags: rust crates-io naming availability check
---

# Skill: crates-io-name-check

Verify that a new Rust crate name is available and appropriate on crates.io.

## Purpose

Verify that a new Rust crate name is **available and appropriate** on crates.io before
committing to it in `Cargo.toml`. This is a generic skill for any Rust project using
this template — run it whenever you create a new crate (workspace member or standalone).

## Trigger Conditions

- When creating a new crate under `crates/` in the workspace
- When starting a new Rust project from this template
- Before the first `cargo publish`
- When renaming a crate

## Prerequisites

- Access to crates.io (internet connection)
- `cargo` CLI installed (for `cargo search`)

## Availability Check

### Method 1: cargo search (CLI, recommended)

```bash
cargo search <your-crate-name>
```

- If the **exact name** appears in results → **taken**, choose another.
- No exact match → likely available (confirm with Method 2).

### Method 2: crates.io API

```bash
STATUS=$(curl -s -o /dev/null -w "%{http_code}" https://crates.io/api/v1/crates/<your-crate-name>)
if [ "$STATUS" = "404" ]; then
  echo "✓ Name is available"
else
  echo "✗ Name is taken (HTTP $STATUS)"
fi
```

### Method 3: Browser

Open `https://crates.io/crates/<your-crate-name>` — 404 page = available.

## Naming Best Practices

| Rule | Good | Bad |
|------|------|-----|
| Use `kebab-case` | `my-tool-core` | `my_tool_core` |
| Be specific | `invoice-parser` | `parser` |
| Avoid generic names | — | `utils`, `helpers`, `common` |
| Scope to project | `myapp-cli`, `myapp-sdk` | `cli`, `sdk` |
| Keep it short | `foobar-rs` | `foobar-rust-library-2026` |
| No name squatting | — | reserving empty crates |

## Similarity Check

Also check for confusingly similar names that could mislead users:

```bash
# Search for similar prefixes
cargo search <prefix>

# Check typosquat risk manually on crates.io
# https://crates.io/search?q=<your-name>
```

## Common Issues

### Name appears taken but project is abandoned

**Symptom**: Crate exists on crates.io but has no recent updates or downloads  
**Fix**: Contact the owner via crates.io contact link, or choose a different name with a suffix like `-rs` or `-core`

### False positive from cargo search

**Symptom**: `cargo search` shows similar names but not exact match  
**Fix**: Verify with Method 2 (API) or Method 3 (browser) - only exact matches block your name

## Cargo.toml Name vs Package Directory

Note that the `[package] name` in `Cargo.toml` is the **published crate name**.
The directory name under `crates/` can differ but should match for clarity:

```
crates/
  my-tool-core/    ← directory
    Cargo.toml     → [package] name = "my-tool-core"  ✓
```

## Template Usage Note

The template includes a placeholder crate `crates/example-crate/`. When you replace
it with your real crate, **rename both the directory and the `[package] name`** in
`Cargo.toml`, and run this skill to verify availability before your first commit.

## References

- [crates.io policies](https://crates.io/policies)
- [Cargo naming guidelines](https://doc.rust-lang.org/cargo/reference/manifest.html#the-name-field)
- [crates.io search](https://crates.io/search)
