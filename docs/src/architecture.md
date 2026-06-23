# Architecture

## Overview Diagram

A human-friendly overview of the project — what it is, how to get started, and what's inside.

![Overview](overview.svg)

## Technical Architecture

The detailed system architecture showing crate dependencies, pipeline stages, skills, and data flow.

![System Architecture](architecture.svg)

## What the Diagram Shows

- **Pipeline Orchestration** — CI/CD stages from analysis to deployment with timing info
- **Workspace Topology** — All crates organized by layer with purpose descriptions
- **Active Skills** — Agent skills for development workflows
- **Interface Protocols** — Slash commands and tool integrations

## Crate Layers

| Layer | Crates | Purpose |
|-------|--------|---------|
| Applications | `sample-app` | Reference binary application demonstrating the workspace template |
| Core Libraries | `benchmarks` | Performance measurement and Criterion benchmarks |
| Templates | `actor-runtime-template` | Actor runtime with supervision and restart strategies |
| | `checkpoint-template` | Serializable state with versioning and file storage |
| | `hybrid-storage-template` | Backend abstraction with feature-gated implementations |
| | `mcp-server-template` | MCP server with tool trait and registry dispatch |
| | `rust-2026-template` | Main workspace template with modern tooling and CI/CD |
| Examples | `example-crate` | Basic example crate |
| | `example-registry-pattern` | Registry dispatch pattern example |
| | `example-storage-pattern` | Storage abstraction pattern example |
| | `hello-world-example` | Simple hello world example |

## Dependency Flow

Dependencies flow upward only: examples → templates → core → applications. This is enforced by `cargo deny`.

## Regenerating the Diagrams

```bash
# Architecture diagram (Excalidraw source + SVG export)
python .agents/skills/architecture-diagram/scripts/generate_diagram.py \
  --root . --out .template/architecture.excalidraw --svg-out .template/architecture.svg

# Overview infographic
python .agents/skills/architecture-diagram/scripts/generate_overview.py \
  --root . --out .template/overview.excalidraw --svg-out .template/overview.svg

# Sync to docs
cp .template/architecture.svg docs/src/architecture.svg
cp .template/overview.svg docs/src/overview.svg
```

## Source Files

| File | Format | Purpose |
|------|--------|---------|
| `.template/architecture.excalidraw` | Excalidraw | Editable source for technical diagram |
| `.template/architecture.svg` | SVG | Published artifact for README and docs |
| `.template/overview.excalidraw` | Excalidraw | Editable source for overview infographic |
| `.template/overview.svg` | SVG | Published artifact for docs |
