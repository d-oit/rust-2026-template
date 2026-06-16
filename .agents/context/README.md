# Cross-Repo Agent Context

This directory provides cross-repository context for AI agents working in derived repositories. It enables agents to discover shared conventions, related repositories, and canonical skill sources.

## Purpose

When multiple repositories share the same template or operating model, agents working in one repo should be aware of patterns, conventions, and knowledge from related projects. This directory breaks down silo-repo boundaries.

## Files

| File | Purpose |
|------|---------|
| `external-repos.json` | Links to related repositories and their agent context |
| `shared-conventions.md` | Cross-repo coding conventions for agents |

## Usage

Agents should check this directory at session start to understand the broader repository ecosystem. The `skill-creator` skill checks `external-repos.json` before creating new skills to avoid duplicates.

## Merge Precedence

When instructions conflict between sources, apply this precedence:

1. **Local repo instructions** (AGENTS.md, .agents/skills/) — highest priority
2. **Imported context** (.agents/context/) — secondary reference
3. **Template defaults** (upstream rust-2026-template) — fallback only

## Maintenance

- Update `external-repos.json` when adding new related repositories
- Keep `shared-conventions.md` in sync across all derived repos
- All entries should include `last_verified` timestamps for freshness tracking
