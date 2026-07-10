# Template patterns — which crate to copy

This template ships **optional** pattern crates. For most new projects, keep one
application crate and delete the rest (`./scripts/init-template.sh --minimal`).

| Pattern | Crate | Copy when… |
|---------|-------|------------|
| **Trait-only storage** | `example-storage-pattern` | You want a storage trait + mock tests without SQL deps |
| **Hybrid storage wrapper** | `hybrid-storage-template` | You need `HybridStorage` + working `MemoryBackend` |
| Registry dispatch | `example-registry-pattern` | Handler map by name |
| Actor mailbox / supervision | `actor-runtime-template` | Message-passing concurrency (hand-rolled, not ractor) |
| Checkpoints / migrations | `checkpoint-template` | Serializable state + version migrations |
| MCP tool registry | `mcp-server-template` | Tool trait + registry dispatch |

### Storage specifically

1. **Default choice:** copy `example-storage-pattern` (trait + mock).
2. **In-memory multi-backend demo:** use `hybrid-storage-template` with `MemoryBackend`.
3. **Do not** treat `SqliteBackend` as production code — it is a fail-closed stub under the optional `sqlite` feature.

See also: [trait-only-storage.md](./trait-only-storage.md), [registry-dispatch.md](./registry-dispatch.md).
