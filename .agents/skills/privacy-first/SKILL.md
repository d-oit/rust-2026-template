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
  category: security
  author: d-oit
  version: "1.0"
  adapted-from: d-o-hub/github-template-ai-agents
  tags: privacy security email lint quality personal-data rust cargo
---

# Privacy First

## When to Activate

- User asks: "prevent emails", "remove personal data", "privacy check", "never use email"
- Before writing any new code, `Cargo.toml`, config, or documentation file
- During code review or quality gate checks
- When adding contact information to any file

## Agent Workflow

### 1. Before Writing Any File

Never introduce an email address yourself — not in code, config, docs,
comments, or commit examples. The pre-commit hook and quality gate (see
Existing Enforcement) will catch Rust files and tracked files respectively,
but a gate catching your mistake still means the mistake was made:

```bash
# Re-check a file you just edited — the same pattern the gate uses
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

## Existing Enforcement (run these, don't reinvent)

Privacy scanning is already wired into this repository — do not paste a new
grep snippet into `scripts/quality-gates.sh`; that file is a thin wrapper around
`cargo xtask quality run`, and the Privacy Check lives in
`crates/xtask/src/quality_helpers.rs` (`run_privacy_check`).

```bash
# Run only the Privacy Check (fast feedback)
bash scripts/quality-gates.sh --only privacy

# It also runs inside the full gate, on every tier used for PRs/releases
bash scripts/quality-gates.sh
```

Two scanners already run automatically:

- **Pre-commit** — `.githooks/pre-commit` (installed by `./scripts/bootstrap.sh`
  via `core.hooksPath`) greps staged `.rs` files for emails, excluding
  `example.com`/`example.org`/`test.com`. Rust files only; config files reach
  the next scanner.
- **Quality gate** — `run_privacy_check` scans every git-tracked file (all
  file types) for email matches, failing the gate on any non-test-domain hit.
  It is part of the `pull-request`, `protected-branch`, and `release` tiers.

If a scan flags a legitimate test fixture, prefer keeping the privacy rule and
fixing the fixture (use `example.com`) over widening the exclude pattern.

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

## Rationalizations

| Rationalization | Reality |
|-----------------|---------|
| "It's just a test email, not real." | Test emails in non-test code set a bad precedent. Use `example.com` only in `#[cfg(test)]`. |
| "The email is in a comment, not code." | Comments are copied. Emails in comments end up in copy-paste across the codebase. |
| "We need a contact email somewhere." | Use GitHub Issues URLs, not personal emails. |

## Red Flags

- [ ] Email addresses in `Cargo.toml` authors field
- [ ] Personal emails in `README.md` or `CONTRIBUTING.md`
- [ ] Real emails in Rust source code (not in `#[cfg(test)]` blocks)
