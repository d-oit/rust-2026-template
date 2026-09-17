# Quality Gate Run Summary

**Timestamp:** 2026-09-17T16:32:28Z
**Commit:** b5076dc16a2fb23c2db3a995e76ad2bb1eb9e0cb
**Branch:** jules-12939407564633582191-ce7b7a74

| Check | Status | Details |
|---|---|---|
| LOC Limits | ✅ success | Passed in 14.088015ms |
| Rust Format | ✅ success | Passed in 388.629743ms |
| Rust Clippy | ✅ success | Passed in 1.749513317s |
| Rust Build | ✅ success | Passed in 2.660212342s |
| Rust Tests | ✅ success | Passed in 1.620059939s |
| Rust Doc Tests | ✅ success | Passed in 301.461271ms |
| Rust Security Audit | ❌ failed | Missing tool: 'cargo-audit'. Guidance: install with `cargo install cargo-audit --locked` (required by tier 'protected-branch'; the gate fails closed instead of skipping) |
| Rust Dependency Policy (Deny) | ❌ failed | Missing tool: 'cargo-deny'. Guidance: install with `cargo install cargo-deny --locked` (required by tier 'protected-branch'; the gate fails closed instead of skipping) |
| Rust Unused Dependencies (Machete) | ❌ failed | Missing tool: 'cargo-machete'. Guidance: install with `cargo install cargo-machete --locked` (required by tier 'protected-branch'; the gate fails closed instead of skipping) |
| Rust MSRV Audit | ✅ success | Passed in 811.407076ms |
| Shell Script Lint (ShellCheck) | ❌ failed | Missing tool: 'shellcheck'. Guidance: install shellcheck (e.g. `apt-get install shellcheck` or `brew install shellcheck`) (required by tier 'protected-branch'; the gate fails closed instead of skipping) |
| Markdown Lint (markdownlint-cli2) | ❌ failed | Missing tool: 'markdownlint-cli2'. Guidance: install with `npm install -g markdownlint-cli2` (required by tier 'protected-branch'; the gate fails closed instead of skipping) |
| Privacy Check (No emails) | ✅ success | Passed in 44.572025ms |
| Secret Scan | ✅ success | Passed in 75.726114ms |
| GitHub Actions Workflow Validation | ✅ success | Passed in 5.440955147s |
| CI Status Artifact Check | ✅ success | Passed in 248.619µs |

## Overall: **FAILURE**
