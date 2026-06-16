---
name: crates-io-name-check
description: >
  Verify that a new Rust crate name is available and appropriate on crates.io
  before committing to it. Use when creating a new crate, starting a new Rust project,
  or before the first cargo publish.
  Triggers: "check crate name", "is name taken", "crates.io available", "new crate".
category: rust
license: MIT
metadata:
  author: d-oit
  version: "1.0"
  tags: rust crates naming publish
---

# Skill: crates-io-name-check

## Purpose

Verify that a new Rust crate name is **available and appropriate** on crates.io before
committing to it in `Cargo.toml`. This is a generic skill for any Rust project using
this template — run it whenever you create a new crate (workspace member or standalone).

## When to Use

- Creating a new crate under `crates/` in the workspace
- Starting a new Rust project from this template
- Before the first `cargo publish`
- When renaming a crate

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

## Rationalizations

| Rationalization | Reality |
|-----------------|---------|
| "The name sounds fine, I don't need to check." | Names are permanent on crates.io. Verify before first publish. |
| "I'll rename later if needed." | Renaming a published crate breaks all downstream users. |
| "Similar names are fine." | Confusingly similar names lead to typosquatting and user confusion. |

## Red Flags

- [ ] Publishing without checking name availability first
- [ ] Using generic names like `utils`, `helpers`, `common`
- [ ] Ignoring similar existing crate names that could confuse users

## References

- [crates.io policies](https://crates.io/policies)
- [Cargo naming guidelines](https://doc.rust-lang.org/cargo/reference/manifest.html#the-name-field)
- [crates.io search](https://crates.io/search)
