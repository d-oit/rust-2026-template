# 0003: Template Crate Patterns

## Status

Proposed

## Context

The rust-2026-template repository serves as a foundation for new Rust projects. Analysis of production codebases (chaotic_semantic_memory, rust-self-learning-memory, axocoatl) revealed recurring architectural patterns that should be captured as reusable templates.

## Decision

Add four template crates demonstrating core architectural patterns:

### MCP Server Template (`crates/mcp-server-template/`)

Pattern: Tool trait with registry-based dispatch
- Tool trait with lifecycle hooks (`init`, `validate`, `handle`)
- Registry pattern using `HashMap<&'static str, Arc<dyn Tool>>`
- Request/response types with JSON validation

### Actor Runtime Template (`crates/actor-runtime-template/`)

Pattern: Message-passing actor model
- ActorMessage enum for mailbox communication
- ActorState trait for validated state transitions
- RestartStrategy enum (Always, Backoff, Never, OnError)
- Supervisor with spawn capabilities

### Checkpoint Template (`crates/checkpoint-template/`)

Pattern: Serializable state with versioning
- CheckpointHeader with version and timestamp
- MigrationRegistry for schema evolution
- FileStorage with atomic writes via temp file rename

### Hybrid Storage Template (`crates/hybrid-storage-template/`)

Pattern: Backend abstraction with feature-gated implementations
- Backend trait (get, set, delete, list_keys)
- MemoryBackend for tests
- KvBackend using in-memory map
- SqliteBackend placeholder for libSQL/Turso

## Consequences

- Templates provide copy-paste starting points for common patterns
- All crates follow workspace conventions (≤500 LOC, doc comments, tests)
- Templates demonstrate real-world patterns without production coupling
- Clean separation between pattern and implementation