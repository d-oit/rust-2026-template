# Security Policy

## Supported Versions

This is a **template repository**. Security fixes are applied to the `main` branch only.
When you use this template to create a new project, you are responsible for applying
security updates to your fork.

| Version | Supported |
| ------- | --------- |
| latest `main` | ✅ |
| older snapshots | ❌ |

## Reporting a Vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Report vulnerabilities by opening a
[GitHub Security Advisory](https://github.com/d-oit/rust-2026-template/security/advisories/new).

Please include:

- A description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

You will receive an acknowledgment within 48 hours and a full response within 7 days.

## Rust-Specific Security Practices

This template enforces the following security practices:

### Dependency Auditing

```bash
# Check for known vulnerabilities in dependencies
cargo audit

# Enforce license and supply chain policy
cargo deny check
```

Both run automatically in CI on every push.

### Supply Chain Security

- `deny.toml` configures `cargo-deny` with:
  - Allowed licenses list
  - Banned crates list
  - Advisory database checks
- Dependabot is configured to auto-update dependencies weekly

### Unsafe Code

- All `unsafe` blocks must include a `// SAFETY:` comment explaining invariants
- Avoid `unsafe` unless absolutely necessary
- If using `unsafe`, document it in your crate's `lib.rs` with `#![forbid(unsafe_code)]`
  or explicitly allow and document each usage

### Secrets and Credentials

- Never commit secrets, tokens, API keys, or credentials
- Use environment variables or secret management systems
- The `privacy-first` skill in `.agents/skills/privacy-first/` enforces no email
  addresses in the codebase

## When Using This Template

After creating a project from this template:

1. Update this `SECURITY.md` with your project's security contact
2. Enable GitHub's Dependabot alerts in your repo settings
3. Configure branch protection on `main`
4. Review `deny.toml` and customize for your license requirements
