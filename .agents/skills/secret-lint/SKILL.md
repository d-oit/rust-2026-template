---
name: secret-lint
description: Automated secret scanning using secretlint to prevent credential leaks.
category: security
---

# Secret Lint Skill

This skill provides automated secret scanning using [secretlint](https://github.com/secretlint/secretlint).

## Usage

### Local Scanning

Run the following command from the repository root:

```bash
make secrets-lint
```

### Pre-commit Hook

The skill is integrated into the pre-commit pipeline and will scan modified files before each commit.

### CI Integration

The `secretlint.yml` workflow runs this skill on every push to `main` and `develop` branches.

## Configuration

- `.secretlintrc.json`: Ruleset configuration (at root).
- `.secretlintignore`: Ignore patterns (at root).
- `package.json`: Dependencies and scripts (in this folder).

## Rationalizations

| Rationalization | Reality |
|---|---|
| "It's only a test secret, so it's fine to commit." | Test secrets can leak patterns or trigger scanner false alarms. Use fixture mocks or ignore rules. |
| "I'll clean up the commit history later." | Once pushed, secrets are compromised. Prevent them from entering git in the first place. |
| "I can bypass pre-commit with --no-verify." | Bypassing pre-commit bypasses secret checks and risks leaking production credentials. |

## Red Flags

- [ ] Using `--no-verify` to bypass secret scanning
- [ ] Committing hardcoded credentials or API keys directly to git
- [ ] Disabling secretlint rules globally instead of configuring `.secretlintignore`
