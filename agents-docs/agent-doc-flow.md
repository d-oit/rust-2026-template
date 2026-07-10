# Agent Documentation Flow

This repository follows a **3-layer canonical documentation model** to ensure AI agents have high-fidelity, non-redundant project context.

## 1. Canonical Agent Contract (`AGENTS.md`)

- **Role**: Single Source of Truth (SSOT).
- **Contents**: All repo-wide rules, coding constraints, security invariants, quality gates, and high-level workflows.
- **Constraint**: Must be kept under 200 lines of code (LOC) to remain token-efficient.

## 2. Reusable Procedures (`.agents/skills/`)

- **Role**: Executable task knowledge.
- **Contents**: Step-by-step runbooks for specific tasks (e.g., `release-rust`, `lint-rust`).
- **Usage**: Referenced by name in `AGENTS.md` and tool adapters.

## 3. Tool Adapters (`CLAUDE.md`, `GEMINI.md`, etc.)

- **Role**: Platform-specific deltas.
- **Contents**: Hardware/harness differences, unique tool integrations, and context-loading quirks.
- **Syntax**: Uses `@AGENTS.md` to point to the canonical contract.

### Standard Header

Each tool adapter must include this standard header:

```markdown
# <Tool> Adapter
# Canonical project rules live in AGENTS.md (max 200 LOC)
# This file contains ONLY tool-specific differences
# Do not duplicate repo-wide instructions here
```

## Validation

CI automatically validates the integrity of agent entrypoints. The validation script `scripts/validate-agent-entrypoints.sh` ensures:

1. Required tool files exist.
2. They follow the thin adapter pattern (starting with `@AGENTS.md`).
3. No duplicated project-wide instruction content is introduced outside of `AGENTS.md`.

## Maintainer Guidance

- **To update project rules**: Edit `AGENTS.md`.
- **To add a workflow**: Create a new skill in `.agents/skills/`.
- **To add a tool**: Create a new adapter file (e.g., `MYTOOL.md`) and register it in `scripts/validate-agent-entrypoints.sh`.
