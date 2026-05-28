# Codacy PR Analysis Output Format

The `codacy pull-request` command with `--output json` produces a JSON object containing analysis results.

## Key Fields

- `newIssues`: Array of issues introduced in the PR.
- `fixedIssues`: Array of issues resolved in the PR.
- `qualityGateStatus`: `Passed`, `Warning`, or `Failed`.

## Issue Object Schema

```json
{
  "resultDataId": 123456789,
  "hash": "abc123def456...",
  "message": "Avoid using 'eval()'",
  "file": "src/app.js",
  "line": 42,
  "tool": "ESLint",
  "severity": "Critical"
}
```

### Important: resultDataId

When suppressing issues via the CLI (e.g., `--ignore-issue`), you **MUST** use the numeric `resultDataId`. The `hash` string is used for identification in the UI but is NOT supported by the suppression command.
