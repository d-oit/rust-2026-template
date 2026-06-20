# Architecture

![System Architecture](architecture.svg)

## Overview

This diagram shows the complete system architecture of the Rust 2026 Template workspace, including:

- **Pipeline Orchestration** — CI/CD stages from analysis to deployment
- **Workspace Topology** — All 11 crates organized by layer (applications, core libraries, templates, examples)
- **Active Skills** — 21 agent skills for development workflows
- **Interface Protocols** — Slash commands and tool integrations

## Crate Layers

| Layer | Crates | Purpose |
|-------|--------|---------|
| Applications | `sample-app` | Reference binary application |
| Core Libraries | `benchmarks` | Performance measurement |
| Templates | `actor-runtime-template`, `checkpoint-template`, `hybrid-storage-template`, `mcp-server-template`, `rust-2026-template` | Reusable architectural patterns |
| Examples | `example-crate`, `example-registry-pattern`, `example-storage-pattern`, `hello-world-example` | Learning references |

## Dependency Flow

Dependencies flow upward only: examples → templates → core → applications. This is enforced by `cargo deny`.

## Regenerating the Diagram

```bash
python .agents/skills/architecture-diagram/scripts/generate_diagram.py --root . --out .template/architecture.svg
cp .template/architecture.svg docs/src/architecture.svg
```
