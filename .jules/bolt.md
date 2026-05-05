## 2025-05-15 - [Tokio Runtime and Tracing Metadata in CLI]

**Learning:** Default multi-threaded Tokio runtime and full tracing metadata collection introduce significant overhead in CLI tools that do not require high concurrency. Switching to the `current_thread` flavor and disabling thread IDs/names in tracing reduces startup time and improves throughput for simple I/O or batch tasks.
**Action:** Always prefer `#[tokio::main(flavor = "current_thread")]` for CLI applications and disable redundant tracing metadata unless the tool is explicitly designed for high-concurrency environments.

## 2025-05-15 - [Linker Dependencies in Template Repos]

**Learning:** Template repositories often include aggressive build optimizations like `mold` which may not be present in all development environments (like restricted sandboxes).
**Action:** Verify the presence of specialized tools before assuming their availability in the build configuration, and provide fallbacks or clear documentation for local environment adjustments.

## 2025-05-20 - [Efficient Workspace Scanning]

**Learning:** Recursive scans (like `grep -r`) in a Rust workspace are surprisingly expensive due to the `target/` directory, which often contains tens of thousands of build artifacts. Filtering results with `grep -v` after a full scan is a common anti-pattern that wastes significant I/O.
**Action:** Always use `--exclude-dir=target` (and `.git`) in search commands to prevent the tool from even entering these directories. This can reduce scan times by 70% or more.

## 2026-05-05 - [Documentation Architecture for Agent Performance]

**Learning:** Mixing human-centric narrative with agent-specific technical instructions in a single README.md leads to "prompt spillover," where agents consume unnecessary tokens and may misinterpret conversational text as strict constraints.
**Action:** Establish a clear separation: README.md for humans, AGENTS.md as the canonical technical source of truth for agents. Use thin wrappers in tool-specific files (CLAUDE.md, .cursor/rules.md, etc.) that point to AGENTS.md to minimize redundancy and prevent "prompt drift" across different AI assistants.

## 2026-05-05 - [CI Rigidity in Documentation]

**Learning:** Automated linting for documentation (markdownlint) and commit messages (commitlint) can block development if not accounted for in agent workflows. MD031 (blanks around fences) and body-max-line-length are common failure points.
**Action:** Explicitly document CI-enforced documentation rules in AGENTS.md and conventions.md. Agents must verify documentation changes against local linting tools (npx markdownlint-cli2) and ensure commit bodies are properly wrapped to avoid build failures.
