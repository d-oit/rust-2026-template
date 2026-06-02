## Workspace Layering

Crates in this workspace follow a strict upward-only dependency direction:

| Layer | Crate suffix | May depend on |
|---|---|---|
| Domain types | `*-types`, `*-domain` | external crates only |
| Core logic | `*-core`, `*-logic` | layer above |
| Adapters / backends | `*-adapters`, `*-db`, `*-http` | layers above |
| Entry points | `*-cli`, `*-bin`, `*-mcp` | all layers above |

**Rule:** A crate at layer N must never import a crate at layer N+1 or higher.
