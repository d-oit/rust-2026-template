---
name: architecture-diagram
description: Generate a human-friendly overview infographic by scanning the live project structure, including workspace crates, skills, and scripts. Use this skill whenever the user asks to regenerate, refresh, or update the overview diagram. Triggers on phrases like "update the overview", "regenerate the overview SVG", or "sync the overview".
license: MIT
metadata:
  category: documentation
  author: d-oit
  version: "1.0"
  platform: agentskills.io
---

# Overview Diagram

Generates a human-friendly overview infographic showing what the project is, how to get started, what's inside, and how it all connects.

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
- **Quality checks** → `config/xtask.json` tier check lists
- **CI jobs** → job IDs across `.github/workflows/*.yml`

### Step 3 — Sync to Docs

```bash
cp .template/overview.svg docs/src/overview.svg
```

`scripts/sync-architecture.sh` performs steps 2 and 3 in one shot and is what
the `Update Overview Diagram` workflow runs.

### Step 4 — Confirm and Report

After the script exits:
1. Tell the user the output path.
2. Report counts: N crates · M skills.
3. If counts differ from the last known state, summarize what changed.

## Output

- `.template/overview.excalidraw` — editable Excalidraw source
- `.template/overview.svg` — generated SVG for docs embedding

## What the stats mean

The "BY THE NUMBERS" cards are derived, never hand-written. If a number looks
wrong, fix the discovery logic rather than editing the generated files.

| Stat | Source |
|------|--------|
| Workspace Members | `cargo metadata` workspace members (includes `examples/*` and `benchmarks`) |
| Crates | directories under `crates/` that contain a `Cargo.toml` |
| AI Skills | `.agents/skills/*/SKILL.md` |
| CI Jobs | unique job IDs across `.github/workflows/*.yml` |
| Quality Checks | union of the `checks` lists for every tier in `config/xtask.json` |
| Shell scripts | `scripts/*.sh` |

`Quality Checks` previously scraped `scripts/quality-gates.sh` for `cargo`
invocations. That script is now a thin wrapper around `cargo xtask`, so the
count collapsed to `1` while the real gate ran nineteen checks. Do not
reintroduce command-scraping for this number.

## Optional Dependencies

### Node.js (for SVG/PNG export)

Required for SVG/PNG export from Excalidraw:

```bash
npm ci
```

Without Node.js, use `--no-export` to generate only the `.excalidraw` source.
Never commit a partially regenerated diagram: if export fails, the stale SVG
stays in place and silently keeps advertising old counts. Check that the
`EXPORTED:` lines appear before committing.

`cargo` must be on `PATH` for accurate crate discovery. Without it the script
falls back to parsing `Cargo.toml` globs and prints a warning — that path now
reports the same totals, but the warning is a signal that the run is degraded.

## CLI Flags

| Flag | Description |
|------|-------------|
| `--root PATH` | Workspace root (default: `.`) |
| `--out PATH` | Excalidraw output path (default: `.template/overview.excalidraw`) |
| `--svg-out PATH` | SVG export path (default: `.template/overview.svg`) |
| `--png-out PATH` | PNG export path (optional, not tracked) |
| `--no-export` | Skip SVG/PNG export (Excalidraw only) |

## Rationalizations

| Rationalization | Reality |
|-----------------|---------|
| "The overview is close enough, no need to regenerate" | Stale diagrams mislead new contributors about project structure. |
| "I'll update the diagram manually" | Manual edits break on next regeneration and introduce inconsistencies. |

## Red Flags

- [ ] Committing workspace changes without regenerating the overview.
- [ ] Manually editing the SVG instead of using the generator script.
- [ ] Direct push failures caused by missing retry loops or unhandled concurrent updates on main (see case `update-diagram-push-20260917` in `.agents/ci/regression-matrix.json`).

## Why the PNG is not committed

PNG export rasterises through the local font stack, so the same diagram yields
different bytes on a CI runner than on a developer machine. The SVG and
Excalidraw outputs are byte-identical everywhere. Because nothing embeds the
PNG — the README and docs both use `overview.svg` — committing it produced a
spurious "regenerate overview diagram" PR on every workflow run. Generate one
ad hoc with `--png-out` when you need a raster preview; do not commit it.
