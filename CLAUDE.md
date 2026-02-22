@AGENTS.md

<!-- Claude Code reads AGENTS.md directly via @-reference above.
     Add Claude-specific overrides below if needed. -->

## Claude-Specific Notes

- Use `TodoWrite` tool to track task progress
- Prefer `Read` tool over `Bash cat` for file inspection
- Use `Bash` with `cargo nextest run -p <crate>` for targeted tests
- Always run `./scripts/quality-gates.sh` before marking a task complete
- For WSL2: watch for file watcher issues - check `.vscode/settings.json`
