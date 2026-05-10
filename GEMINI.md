@AGENTS.md

<!-- Gemini CLI reads AGENTS.md directly via @-reference above.
     See AGENTS.md for canonical repository guidance. -->

## Gemini-Specific Tips

- Use `google_web_search` to stay updated on the latest Rust ecosystem developments and crate documentation.
- Leverage parallel `ReadFile` calls when inspecting multiple related files.
- Always verify large-scale refactors with `cargo check --workspace` early in the process.
- Use `./scripts/quality-gates.sh` as your final validation gate before completion.
