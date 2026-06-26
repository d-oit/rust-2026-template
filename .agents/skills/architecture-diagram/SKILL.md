---
name: overview-diagram
version: 1.0.0
description: Generate a human-friendly overview infographic by scanning the live project structure, including workspace crates, skills, and scripts. Use this skill whenever the user asks to regenerate, refresh, or update the overview diagram. Triggers on phrases like "update the overview", "regenerate the overview SVG", or "sync the overview".
category: documentation
license: MIT
metadata:
  author: d-oit
  version: "1.0"
  platform: agentskills.io
---

# Overview Diagram

Generates a human-friendly overview infographic showing what the project is, how to get started, what's inside, and how it all connects.

## When to Use

- User asks to update / regenerate / sync the overview diagram.
- Crates, skills, scripts, or workflows have changed.
- First-time setup (diagram doesn't exist yet).

## Execution

### Step 1 — Locate Project Root

```bash
ls .agents/ Cargo.toml 2>/dev/null || echo "NOT_FOUND"
```

### Step 2 — Generate the Diagram

```bash
python .agents/skills/architecture-diagram/scripts/generate_overview.py \
  --root . \
  --out .template/overview.excalidraw \
  --svg-out .template/overview.svg
```

The script auto-discovers at runtime:
- **Workspace crates** → `cargo metadata` (with Cargo.toml glob fallback)
- **AI skills** → `.agents/skills/*/` directories
- **Commands** → `.opencode/commands/*.md`

### Step 3 — Sync to Docs

```bash
cp .template/overview.svg docs/src/overview.svg
```

### Step 4 — Confirm and Report

After the script exits:
1. Tell the user the output path.
2. Report counts: N crates · M skills.
3. If counts differ from the last known state, summarize what changed.

## Output

- `.template/overview.excalidraw` — editable Excalidraw source
- `.template/overview.svg` — generated SVG for docs embedding

## Optional Dependencies

### Node.js (for SVG export)

Required for SVG/PNG export from Excalidraw:

```bash
npm ci
```

Without Node.js, use `--no-export` to generate only the `.excalidraw` source.

## CLI Flags

| Flag | Description |
|------|-------------|
| `--root PATH` | Workspace root (default: `.`) |
| `--out PATH` | Excalidraw output path (default: `.template/overview.excalidraw`) |
| `--svg-out PATH` | SVG export path (default: `.template/overview.svg`) |
| `--png-out PATH` | PNG export path (optional) |
| `--no-export` | Skip SVG/PNG export (Excalidraw only) |

## Rationalizations

| Rationalization | Reality |
|-----------------|---------|
| "The overview is close enough, no need to regenerate" | Stale diagrams mislead new contributors about project structure. |
| "I'll update the diagram manually" | Manual edits break on next regeneration and introduce inconsistencies. |

## Red Flags

- [ ] Committing workspace changes without regenerating the overview.
- [ ] Manually editing the SVG instead of using the generator script.
