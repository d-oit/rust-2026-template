@AGENTS.md

## Tool-Specific Guidance

- Use the `TodoWrite` tool to track task progress during complex refactors.
- Prefer the `Read` tool over `Bash cat` for efficient file inspection.
- Use `Bash` with `cargo nextest run -p <crate>` for targeted testing of individual workspace members.
- Batch ALL file reads, edits, and Bash commands in ONE message where possible.
- After spawning Task agents: STOP — do not poll status; wait for results.
- Use `Bash` with `cargo nextest run -p <crate>` for targeted per-crate testing.
