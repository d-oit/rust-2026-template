# Gemini Adapter
# Canonical project rules live in AGENTS.md (max 200 LOC)
# This file contains ONLY tool-specific differences
# Do not duplicate repo-wide instructions here

@AGENTS.md

## Tool-Specific Guidance

- Use `google_web_search` to stay updated on the latest Rust ecosystem developments and crate documentation.
- Leverage parallel `ReadFile` calls when inspecting multiple related files.
- Always verify large-scale refactors with `cargo check --workspace` early in the process.
