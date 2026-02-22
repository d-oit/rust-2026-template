@AGENTS.md

<!-- Gemini CLI reads AGENTS.md directly via @-reference above.
     Add Gemini-specific overrides below if needed. -->

## Gemini-Specific Notes

- Use `google_web_search` for researching Rust crates and ecosystem updates
- Prefer reading multiple files with parallel `ReadFile` calls
- Always verify with `cargo check --workspace` before applying large refactors
- For long-running builds: check `CARGO_BUILD_JOBS` env var for parallel limits
- Use `./scripts/quality-gates.sh` as final validation before completing tasks
