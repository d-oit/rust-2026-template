<!-- AUTO-GENERATED — edit src/lib.rs in the crate, not this file -->

# Template patterns — which crate to copy

This template ships **optional** pattern crates and example applications. For most new projects, keep one
application crate and delete the rest (`./scripts/init-template.sh --minimal`).

| Pattern | Crate / Example | Copy when… |
|---------|-----------------|------------|
| **Trait-only storage** | `example-storage-pattern` | You want a storage trait + mock tests without SQL deps |
| **Hybrid storage wrapper** | `hybrid-storage-template` | You need `HybridStorage` + working `MemoryBackend` |
| Registry dispatch | `example-registry-pattern` | Handler map by name |
| Actor mailbox / supervision | `actor-runtime-template` | Message-passing concurrency (hand-rolled, not ractor) |
| Checkpoints / migrations | `checkpoint-template` | Serializable state + version migrations |
| MCP tool registry | `mcp-server-template` | Tool trait + registry dispatch |
| Safe CLI `--dry-run` | `examples/cli_dry_run` | CLI triggers paid APIs or costly/irreversible remote operations |
| **Secret detection corpus** | `examples/security_secret_detector` | You need exact-span secret scanning and an adversarial regression corpus |

## Storage specifically

1. **Default choice:** copy `example-storage-pattern` (trait + mock).
2. **In-memory multi-backend demo:** use `hybrid-storage-template` with `MemoryBackend`.
3. **Do not** treat `SqliteBackend` as production code — it is a fail-closed stub under the optional `sqlite` feature.

See also: [trait-only-storage.md](./trait-only-storage.md), [registry-dispatch.md](./registry-dispatch.md), [cli-dry-run.md](./cli-dry-run.md), [secret-detection-corpus.md](./secret-detection-corpus.md).

## Security boundary guidance

For services and tools implementing network egress or file handling, refer to the opt-in example in `examples/security_egress_and_path_validation/`:
- **Safe egress:** Refuses or revalidates redirects, normalizes IPv4/IPv6 representations, and enforces non-routable IP restrictions.
- **Untrusted paths:** Rejects parent directory traversal (`..`), absolute path components, and control/Bidi characters.
- **SSRF defense notice:** URL string parsing and static IP checks do not replace connection-time socket address validation (DNS rebinding / TOCTOU protection).
