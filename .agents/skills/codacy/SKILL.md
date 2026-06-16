---
name: codacy
description: Use Codacy static analysis CLIs to query PR analysis, triage issues, suppress false positives, and run local analysis. Use when Codacy blocks a PR, when asked to fix Codacy issues, suppress false positives, query PR quality data, or integrate Codacy into CI/CD workflows. Also use when the user mentions "Codacy", "static analysis check", "code quality gate", or "Codacy is failing".
category: quality-gates
license: MIT
metadata:
  author: d-oit
  version: 1.0.0
---

# Codacy Static Analysis

Orchestrate static analysis using Codacy Analysis CLI (local) and Codacy Cloud CLI (remote).

## Installation & Auth

```bash
# Analysis CLI (for local runs)
npm i -g @codacy/analysis-cli

# Cloud CLI (for PR data and suppressions)
npm i -g @codacy/codacy-cloud-cli

export CODACY_API_TOKEN=<your-api-token>
```

## PR Triage Workflow

1. **Get PR analysis**:
   `codacy pull-request gh <org> <repo> <prNumber> --output json > /tmp/codacy-pr.json`

2. **Categorize issues**:
   - False positives → Suppress via Cloud CLI.
   - Real issues → Fix in code.

3. **Suppress false positives**:
   `codacy pull-request gh <org> <repo> <prNumber> --ignore-issue <numeric-resultDataId> --ignore-reason FalsePositive`
   *Note: Use numeric `resultDataId`, NOT hash IDs.*

4. **Fix issues**: Batch fix patterns and verify with local lint/tests.

## Local Analysis

```bash
# Initialize configuration (generates .codacy.yml)
codacy-analysis init --default

# Run local analysis
codacy-analysis analyze --pr --output-format json
```

## Known Limitations

| Tool Category | Status | Note |
|---------------|--------|------|
| JS/TS/Shell | ✅ Works | ESLint9, Stylelint, ShellCheck |
| Rust | ⚠️ Limited | Local analysis uses `jscpd` and `Lizard`; Cloud uses `Opengrep` |
| Python/Ruby | ❌ Fails | Missing runtimes/venv issues |
| Java/PMD | ❌ Fails | Missing Java runtime |

Always cross-reference with Cloud CLI for full PR data.

## Rationalizations

| Rationalization | Reality |
|-----------------|---------|
| "Local analysis shows 0 issues, so we are good." | Analysis CLI has limited local tool support; Cloud CLI is the source of truth. |
| "I'll use the issue hash for suppression." | Codacy CLI requires the numeric `resultDataId` for suppressions. |

## Red Flags

- [ ] Relying solely on local `codacy-analysis` for Rust/Python/Java projects.
- [ ] Attempting to suppress issues without a valid `--ignore-reason`.
- [ ] Ignoring the `resultDataId` field in JSON output in favor of hashes.

## References

- `references/config-format.md` - `.codacy.yml` schema and advanced options
- `references/output-format.md` - JSON schema for PR analysis
- `references/supported-tools.md` - Local vs Cloud tool availability
