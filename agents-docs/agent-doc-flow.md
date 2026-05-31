# Agent Documentation Flow

This repository follows a **single-source-of-truth** model for agent guidance. All canonical instructions for AI agents are centralized in `AGENTS.md`.

## Canonical Model

- **Primary Source**: `AGENTS.md` (root directory)
- **Thin References**: `CLAUDE.md`, `GEMINI.md`, `QWEN.md` (root directory)

### Thin Reference Specification

Assistant-specific entrypoints (like `CLAUDE.md`) must be regular files starting with the following directive:

```text
@AGENTS.md
```

This directive tells the respective assistant to read the main guidance file. These files may contain additional assistant-specific tips and tool usage guidelines that are not common to all agents.

## Validation

CI automatically validates the integrity of agent entrypoints. The validation script `scripts/validate-agent-entrypoints.sh` ensures:

1. Required assistant files exist.
2. They contain exactly the minimal forwarding content.
3. No duplicated instruction content is introduced outside of `AGENTS.md`.

## Maintainer Guidance

- **To update agent guidance**: Edit `AGENTS.md` directly.
- **To add a new assistant**: Create a new `<ASSISTANT>.md` file in the root containing exactly `@AGENTS.md` and add it to the `AGENT_FILES` list in `scripts/validate-agent-entrypoints.sh`.
- **Portability**: We use regular files with a text directive instead of filesystem symlinks. This ensures compatibility across all operating systems, git configurations, and archive/export workflows where symlinks might be broken or lost.
