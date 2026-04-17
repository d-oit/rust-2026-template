# Change Workflow

## Golden Path

1. **Read** existing code patterns before modifying
2. **Identify** owner module + relevant file
3. **Add/update tests** first (TDD preferred)
4. `./scripts/code-quality.sh fmt` → fix formatting
5. `cargo clippy --all -- -D warnings` → fix ALL warnings
6. `cargo nextest run -p <package>` → targeted tests
7. `cargo nextest run --all` → full suite
8. `./scripts/quality-gates.sh` → final validation
9. **Commit** with conventional format

## Required Checks Before Commit

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --tests -- -D warnings` (CI parity)
- [ ] `cargo build --workspace`
- [ ] `cargo nextest run --all`
- [ ] `./scripts/quality-gates.sh`

## Skill + CLI Pattern

**Use Skill + CLI first** for high-frequency operations:

| Operation | Skill | Script/CLI |
|-----------|-------|------------|
| Build | `build-rust` | `./scripts/build-rust.sh` |
| Format/Lint | `code-quality` | `./scripts/code-quality.sh` |
| Quality Gates | `code-quality` | `./scripts/quality-gates.sh` |
| CI Issues | `github-workflows` | `gh workflow list` |
| Tests | `test-runner` | `cargo nextest run --all` |
| Debug | `debug-troubleshoot` | `RUST_LOG=debug cargo nextest run` |

**Before using task tool:**
1. Is there a skill in `.agents/skills/`? → Use it
2. Is there a script in `scripts/`? → Use it
3. Is this high-frequency? → Use Skill + CLI
4. Is this complex multi-agent? → Use task tool

## Token Efficiency

**Tool Selection Priority (lowest token cost first):**
1. **Glob** - File discovery
2. **Grep** - Code search
3. **Read** - File inspection
4. **Bash** - Shell commands (expensive - prefer scripts)