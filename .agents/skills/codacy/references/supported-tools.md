# Supported Codacy Tools

Codacy supports hundreds of tools, but only a subset are available in the local Analysis CLI.

## Rust Support

| Tool | Category | Cloud | Local CLI |
|------|----------|-------|-----------|
| Opengrep | Static Analysis | ✅ | ❌ |
| jscpd | Duplication | ✅ | ✅ |
| Lizard | Complexity | ✅ | ✅ |
| Trivy | Security/Vulnerability | ✅ | ✅ |

## Other Common Tools (Local CLI)

| Tool | Language | Status |
|------|----------|--------|
| ESLint9 | JavaScript/TypeScript | ✅ Supported |
| Stylelint | CSS/SCSS | ✅ Supported |
| ShellCheck | Shell | ✅ Supported |
| markdownlint | Markdown | ✅ Supported |
| Hadolint | Dockerfile | ✅ Supported |

Always use `codacy pull-request` to see the full list of issues identified by the Cloud analysis engine.
