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

### Dependency Publication Cooldown

The checked-in `.cargo/config.toml` configures Cargo to wait **three days**
before selecting newly published registry packages during dependency
resolution. This policy requires Cargo 1.100 or newer; older Cargo versions do
not enforce it, so use a current Cargo release when relying on it.

The cooldown applies when Cargo resolves dependencies; it does not retroactively
validate versions already recorded in `Cargo.lock` or protect `cargo install`.
It also requires the registry index to provide publication timestamps, and does
not apply to non-registry sources. It complements rather than replaces
`cargo audit`, `cargo deny`, checksum verification, and lockfile review. It is
a delay to reduce exposure, not a guarantee that a package is safe.

For an urgent update, a one-command override is available:
`CARGO_RESOLVER_INCOMPATIBLE_PUBLISH_AGE=allow cargo update -p <crate> --precise <version>`.
Use it deliberately and review the resulting lockfile.

See the [Cargo registry configuration reference](https://doc.rust-lang.org/cargo/reference/config.html#registry)
and [Rust 1.100 release notes](https://blog.rust-lang.org/2026/11/12/Rust-1.100.0/#cargo-min-publish-age-a-k-a-dependency-cooldowns)
for the supported setting and its behavior.

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
