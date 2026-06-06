# Command: generate-docs

## Description
Generates a Rust-scoped symbol inventory to assist agents in understanding the public API surface of the workspace.

## Execution Protocol
1. **Workspace Discovery**: Identify all workspace members defined in `Cargo.toml` and located in `crates/`.
2. **Symbol Extraction**: For each crate, extract public symbols including:
   - `struct` and `enum` definitions
   - `trait` definitions
   - Public functions (`pub fn`)
   - Associated `///` documentation comments
3. **Documentation Generation**:
   - Primary output: `docs/symbols/` directory.
   - Create one file per crate: `docs/symbols/<crate_name>.md`.
4. **Machine Context**: Trigger `bash scripts/generate-llms-txt.sh` to ensure `llms.txt` and `llms-full.txt` are up-to-date with any new documentation.

## Guidelines
- This command is intended to complement `cargo doc`, providing a more LLM-friendly, flat representation of the codebase.
- Focus on the "what" (public interface) rather than the "how" (internal implementation).
