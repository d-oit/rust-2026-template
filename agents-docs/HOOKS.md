# Agent Hooks

## Overview

Hooks are scripts that run at specific points in the agent lifecycle. They enable automated context injection, validation, and cleanup.

## Available Hooks

### SessionStart Hook

**File**: `hooks/session-start.sh`
**Trigger**: When an agent session begins
**Purpose**: Inject project context into the agent's working memory

The session start hook provides:
- Cargo.toml workspace/package info
- Documentation structure map
- Latest changelog entry
- CI health status (when available)

### Pre-Commit Hook

**File**: `.githooks/pre-commit`
**Trigger**: Before each git commit
**Purpose**: Enforce quality gates before code enters the repository

Checks performed:
- LOC limit enforcement (500 lines per .rs file)
- TOML/YAML syntax validation
- Rust formatting (rustfmt)
- Rust linting (clippy)
- Privacy scan (email addresses)
- Secret scan (API keys, tokens)

### Pre-Push Hook

**File**: `.githooks/pre-push` (if present)
**Trigger**: Before pushing to remote
**Purpose**: Run full quality gate before sharing code

## Creating Custom Hooks

### Hook Script Location

Place hooks in `hooks/` or `.githooks/`:

```bash
#!/usr/bin/env bash
# hooks/my-hook.sh
# Description of what this hook does
set -euo pipefail

# Your hook logic here
```

### Hook Configuration

Register hooks in the appropriate config:

- **Session hooks**: Register in `.claude/settings.json`
- **Git hooks**: Place in `.githooks/` and run `git config core.hooksPath .githooks`
- **Pre-commit framework**: Add to `.pre-commit-config.yaml`

### Hook Best Practices

1. **Idempotent**: Hooks should produce the same result when run multiple times
2. **Fast**: Keep hooks under 10 seconds to avoid blocking workflow
3. **Informative**: Use colors and clear messages for pass/fail/warn
4. **Optional**: Allow skipping with environment variables (e.g., `SKIP_HOOKS=1`)
5. **Logged**: Write hook results to event files for observability

## Hook Integration Points

```
Agent Session Start
    |
    v
hooks/session-start.sh (context injection)
    |
    v
Agent Working...
    |
    v
Pre-Commit (.githooks/pre-commit)
    |
    v
CI Pipeline (.github/workflows/ci.yml)
    |
    v
Post-Release (release-drafter, DORA metrics)
```

## Debugging Hooks

To debug a hook:
1. Run it manually: `bash hooks/session-start.sh`
2. Add `set -x` at the top for trace output
3. Check the hook's exit code: `echo $?`
4. Review hook output for error messages
