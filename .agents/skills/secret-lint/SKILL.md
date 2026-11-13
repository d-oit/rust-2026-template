---
name: secret-lint
description: >
  Automated secret scanning using secretlint to prevent credential leaks.
  Use when the user asks to scan for secrets/credentials/API keys, handle a
  secretlint false positive, or audit a file before committing. Local scanning,
  CI integration, and repository policy for leaked secrets.
  Triggers: "scan for secrets", "check for API keys", "secretlint", "credential leak".
metadata:
  category: security
---

# Secret Lint Skill

Layered secret scanning with [secretlint](https://github.com/secretlint/secretlint)
plus the repository's built-in gate. The local pre-commit hook is homegrown, not
secretlint — see the pre-commit section below.

## Local Scan

There is **no `make secrets-lint` target** — the Makefile has no secrets target.
Run secretlint from the repository root:

```bash
# One-time install (CI does the same via npm ci)
(cd .agents/skills/secret-lint && npm ci)

# Scan everything using the skill package's own script
npm --prefix .agents/skills/secret-lint run secretlint

# Or invoke the binary directly
./.agents/skills/secret-lint/node_modules/.bin/secretlint "**/*"
```

The quality gate has its own passing native secret check (regex-based,
`crates/xtask/src/quality_helpers.rs`) inside `./scripts/quality-gates.sh`:

```bash
bash scripts/quality-gates.sh --only secret
```

## Pre-commit Hook

`.githooks/pre-commit` (installed by `./scripts/bootstrap.sh` via
`core.hooksPath`) runs a regex secret scan — plus an email scan and clippy —
on **staged `.rs` files only** before each commit. Non-Rust files (CI configs,
YAML, JSON) are only covered by secretlint locally and by CI.

## CI Integration

- `secretlint.yml` — runs `secretlint --format junit "**/*"` on pushes and PRs
  targeting `main`/`develop`. Rejects on any finding.
- `security-scan.yml` — `gitleaks detect` on push, PR, and weekly schedule;
  report config lives in `.gitleaks.toml`.
- Regression history for both: `.agents/ci/regression-matrix.json`
  (repro: `gitleaks detect --verbose`).

## Configuration

- `.secretlintrc.json` — secretlint ruleset (repository root).
- `.secretlintignore` — path/pattern exclusions for known fixture files
  (e.g. detector test fixtures). Exclude a *path*, never a *rule*.
- `package.json` **in this folder** (no root `package.json`) holds the
  secretlint dependency and its `secretlint` / `secretlint:ci` scripts. This
  folder is a standalone npm package — run its scripts from inside it
  (`npm --prefix .agents/skills/secret-lint run secretlint`), not via a root
  npm script.

## Handling False Positives

1. Confirm the match is synthetic before exempting anything (fixture, `example.*`
   domain, CI-injected token read).
2. Add the narrowest possible path to `.secretlintignore`, or extend the exclude
   regex for the native gate in `crates/xtask/src/quality_helpers.rs`.
3. Never disable `@secretlint/secretlint-rule-preset-recommend` globally.

## Rotation Rule

A credential that ever reached git is compromised, regardless of whether the
history is rewritten later. Remove it from the tree, **rotate it at the issuer**,
then add the fixture path to `.secretlintignore`.

## Rationalizations

| Rationalization | Reality |
|---|---|
| "It's only a test secret, so it's fine to commit." | Test secrets can leak patterns or trigger scanner false alarms. Use fixture mocks or ignore rules. |
| "I'll clean up the commit history later." | Once pushed, secrets are compromised. Prevent them from entering git in the first place. |
| "I can bypass pre-commit with --no-verify." | Bypassing pre-commit bypasses secret checks and risks leaking production credentials. |
| "The pre-commit hook caught it, so CI will too." | The hook only scans staged `.rs` files; secretlint in CI scans everything. A YAML or workflow file can carry a secret past the hook. |

## Red Flags

- [ ] Using `--no-verify` to bypass secret scanning
- [ ] Committing hardcoded credentials or API keys directly to git
- [ ] Disabling secretlint rules globally instead of configuring `.secretlintignore`
- [ ] Citing `make secrets-lint` or other scan commands not present in the Makefile
- [ ] Excluding a whole fixture directory when one file is the offender