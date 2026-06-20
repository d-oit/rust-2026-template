---
name: architecture-diagram
version: 0.3.0
description: Generate or update a project architecture SVG diagram by scanning the live project structure, including Rust workspace crates and dependencies. Use this skill whenever the user asks to regenerate, refresh, or update the architecture diagram, or when crates, skills, agents, or commands have been added/removed and the diagram is stale. Triggers on phrases like "update the diagram", "regenerate the architecture SVG", "sync the architecture", or "show crate dependencies".
category: documentation
license: MIT
metadata:
  author: d-oit
  version: "1.0"
  platform: agentskills.io
---

# Architecture Diagram

Generates an architecture SVG diagram by scanning the live project structure, including Rust workspace crates, dependencies, and agentic skills.

## When to Use

- User asks to update / regenerate / sync the architecture diagram.
- Crates in the Rust workspace have been added, removed, or their dependencies changed.
- Skills, agents, or commands have changed.
- First-time setup (diagram doesn't exist yet).

## Execution

### Step 1 — Locate Project Root

Use the bash tool to find the project root (directory containing `.agents/` and `Cargo.toml`):

```bash
pwd
ls .agents/ Cargo.toml 2>/dev/null || echo "NOT_FOUND"
```

### Step 2 — Run the Generator Script

Run the script from the project root:

```bash
python .agents/skills/architecture-diagram/scripts/generate_diagram.py \
  --root . \
  --out .template/architecture.svg
```

The script auto-discovers:
- **Rust Workspace** → Parses `Cargo.toml` and uses `cargo metadata` to map crate dependencies and features.
- **Skills** → `.agents/skills/*/SKILL.md` (reads `name:` from frontmatter).
- **Agents** → `.opencode/agents/*.md` (uses filename stem).
- **Commands** → `.opencode/commands/*.md` (uses filename stem, strips leading `/`).

It writes a self-contained SVG to `--out` (default: `.template/architecture.svg`).

### Step 3 — Confirm and Report

After the script exits:
1. Tell the user the output path.
2. Report counts: N crates · M skills · K agents · L commands.
3. If counts differ from the last known state, summarize what changed (e.g., "Added `new-crate`, removed `old-skill`").

## Output

`.template/architecture.svg` — a standalone SVG output file, compatible with GitHub README embedding:
`![Architecture](.template/architecture.svg)`

## Customization

The script reads an optional `docs/diagram-config.json` if present:

```json
{
  "title": "Project Architecture",
  "project_name": "My Project",
  "author": "maintainer",
  "pipeline_stages": [
    {"name": "build", "color": "teal"},
    {"name": "test", "color": "blue"},
    {"name": "deploy", "color": "green"}
  ]
}
```

## Rationalizations

| Rationalization | Reality |
|-----------------|---------|
| "The diagram is close enough, no need to regenerate" | Stale diagrams mislead new contributors and mask architectural drift, especially in complex Rust workspaces. |
| "I'll update the diagram manually in an editor" | Manual SVG edits break on next regeneration and introduce inconsistencies between code and docs. |
| "Architecture diagrams are just decoration" | Diagrams are the primary onboarding tool for understanding system structure and crate layering. |

## Red Flags

- [ ] Committing workspace changes without regenerating the diagram.
- [ ] Manually editing the SVG instead of using the generator script.
- [ ] Ignoring diagram regeneration when adding/removing crates or skills.
