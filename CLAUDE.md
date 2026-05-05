# Claude Code Instructions

Refer to **[AGENTS.md](AGENTS.md)** for the canonical instruction set for this repository.

## Claude-Specific Notes

- Use `TodoWrite` tool to track task progress
- Prefer `Read` tool over `Bash cat` for file inspection
- Always run `./scripts/quality-gates.sh` before marking a task complete
- For WSL2: watch for file watcher issues - check `.vscode/settings.json`
