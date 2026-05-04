---
name: privacy-first
description: >
  Prevent email addresses and personal data from entering the codebase.
  Use when user asks to "prevent emails", "remove personal data", "privacy check",
  "no email", or when writing/editing any Rust code, Cargo.toml, config, or documentation files.
  Also triggers during code review, quality gate checks, or when adding contact information.
license: MIT
compatibility: Works with Claude Code, OpenCode, and similar agents. No external dependencies.
metadata:
  author: d-oit
  version: "1.0"
  adapted-from: d-o-hub/github-template-ai-agents
  tags: privacy security email lint quality personal-data rust cargo
---

# Privacy First

This skill ensures no email addresses or personal data leak into the Rust codebase.
It provides detection, prevention, and automated checking for Rust projects.

## When to Activate

- User asks: "prevent emails", "remove personal data", "privacy check", "never use email"
- Before writing any new code, `Cargo.toml`, config, or documentation file
- During code review or quality gate checks
- When adding contact information to any file

## Agent Workflow

### 1. Before Writing Any File

Before creating or editing any file, check for email patterns:

```bash
# Scan the file being edited for email patterns
grep -E '[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}' <file> || true
```

### 2. Replacement Rules

| Context | Instead of | Use |
|---|---|---|
| `Cargo.toml` authors | `authors = ["Name <email@example.com>"]` | `authors = ["Name"]` |
| README contact | `contact@example.com` | GitHub Issues link |
| CONTRIBUTING.md | `help@example.com` | `https://github.com/owner/repo/issues` |
| Code of Conduct | `conduct@example.com` | "Report via GitHub Issues" |
| Examples in tests | `user@gmail.com` | `user@example.com` (test domain only) |

### 3. Rust-Specific Rules

**Cargo.toml**

```toml
# Bad
[package]
authors = ["Dominik Oswald <user@example.com>"]

# Good
[package]
authors = ["Dominik Oswald"]
```

**README.md / CONTRIBUTING.md**

```markdown
# Bad
Contact: support@example.com

# Good
Report issues: https://github.com/owner/repo/issues
```

**Rust source code**

- Never hardcode email addresses in `const`, `static`, or `fn` bodies
- Use placeholder `example.com` domain only in `#[cfg(test)]` blocks
- Keep `deny.toml` and CI configs free of personal email addresses

### 4. Exceptions (Allowed)

The following are permitted and should NOT be flagged:

- Test data in `tests/` or `#[cfg(test)]` using standard test domains:
  - `example.com`, `example.org`, `test.com`
- URLs in documentation pointing to external services
- Git history (cannot modify)
- Skill reference files with generic examples (for demonstration)

## Quality Gate Integration

Add to `scripts/quality-gates.sh` or CI:

```bash
# Scan for email patterns (exclude test data and git)
EMAIL_PATTERN='[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}'
EXCLUDE_PATTERN='example\.com|example\.org|test\.com|\.git|node_modules|target'

# Check source files
if grep -rE "$EMAIL_PATTERN" \
  --include="*.rs" --include="*.toml" \
  --include="*.yaml" --include="*.json" --include="*.md" . \
  2>/dev/null | grep -vE "$EXCLUDE_PATTERN"; then
  echo "ERROR: Email address detected in codebase"
  exit 1
fi
```

## Quick Reference

**Never do:**

- Add `authors = ["Name <email>"]` in `Cargo.toml`
- Write `contact@example.com` in any markdown
- Use real email addresses in examples
- Include personal emails in `deny.toml` or CI configs

**Always do:**

- Use GitHub Issues URLs for support
- Remove email from `Cargo.toml` authors field
- Use test domains (`example.com`) only in `#[cfg(test)]` blocks
- Link to `SECURITY.md` for vulnerability reporting
