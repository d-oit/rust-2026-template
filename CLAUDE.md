# Claude Adapter
# Canonical project rules live in AGENTS.md (max 200 LOC)
# This file contains ONLY tool-specific differences
# Do not duplicate repo-wide instructions here

@AGENTS.md

## Tool-Specific Guidance

- Use the `TodoWrite` tool to track task progress during complex refactors.
- Prefer the `Read` tool over `Bash cat` for efficient file inspection.
- Use `Bash` with `cargo nextest run -p <crate>` for targeted testing of individual workspace members.
