# Tool Adapters

The repository uses a **3-layer agent architecture**:

1. **Canonical contract** (`AGENTS.md`) — the single source of truth for all AI agents, max 200 LOC.
2. **Tool-specific adapters** (root-level `CLAUDE.md`, `GEMINI.md`, `QWEN.md`) — thin wrappers that reference `AGENTS.md` and add only tool-specific guidance.
3. **Subdirectory adapters** (`.claude/rules.md`, `.gemini/rules.md`, etc.) — CLI-specific rule files that also reference `AGENTS.md`.

## Adapter Manifest

All adapters are declared in `.agents/agent-adapters.toml`. The manifest defines:

- **Contract**: canonical instructions path, skills directory, context files
- **Validation rules**: what the CI gate enforces
- **Adapters**: registered tool-specific adapters with their entrypoints

## Validation

Run the adapter validator locally:

```bash
# Full validation pass
cargo xtask agents validate

# List all registered adapters
cargo xtask agents inventory --format markdown

# Check context files exist
cargo xtask agents check-context
```

CI runs `cargo xtask agents validate` in the `validate-agents` job on every PR that touches agent files.

## Adding a New Adapter

1. Create the adapter file (e.g. `NEW-TOOL.md`) with `@AGENTS.md` as the first content line.
2. Add a `[[adapters]]` entry to `.agents/agent-adapters.toml`.
3. If the tool uses a subdirectory (`.new-tool/rules.md`), add that to the validation script too.
4. Run `cargo xtask agents validate` to confirm.

## Validation Rules

| Rule | Description |
|------|-------------|
| Canonical reference | Each adapter entrypoint must contain `@AGENTS.md` |
| Link verification | Referenced skills, hooks, and context files must exist on disk |
| Policy duplication | Adapters must not contain direct copies of AGENTS.md policy sections |
| Adapter scope | Adapters are limited to tool-specific guidance (command syntax, capability limitations) |
| AGENTS.md LOC | The canonical contract must not exceed 200 lines |
