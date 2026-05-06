@AGENTS.md

<!-- Claude Code reads AGENTS.md directly via @-reference above.
     See AGENTS.md for canonical repository guidance. -->

## Claude-Specific Tips

- Use the `TodoWrite` tool to track task progress during complex refactors.
- Prefer the `Read` tool over `Bash cat` for efficient file inspection.
- Use `Bash` with `cargo nextest run -p <crate>` for targeted testing of individual workspace members.
- Always run `./scripts/quality-gates.sh` before finalizing a task to ensure CI parity.
