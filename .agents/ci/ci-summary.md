# Quality Gate Run Summary

**Timestamp:** 2026-10-17T14:20:49Z
**Commit:** 10cccd918062e4593b0486d9f9508cb7c4a60a3a
**Branch:** jules-14086840885743080200-4eabb65c

| Check | Status | Details |
|---|---|---|
| LOC Limits | ✅ success | Passed in 20.590092ms |
| Rust Format | ✅ success | Passed in 430.848313ms |
| Rust Clippy | ✅ success | Passed in 973.901104ms |
| Rust Build | ✅ success | Passed in 1.487857694s |
| Rust Tests | ✅ success | Passed in 4.501940695s |
| Rust Doc Tests | ✅ success | Passed in 273.067371ms |
| Rust Security Audit | ❌ failed | Missing tool: 'cargo-audit'. Guidance: install with `cargo install cargo-audit --locked` (required by tier 'protected-branch'; the gate fails closed instead of skipping) |
| Rust Dependency Policy (Deny) | ❌ failed | Missing tool: 'cargo-deny'. Guidance: install with `cargo install cargo-deny --locked` (required by tier 'protected-branch'; the gate fails closed instead of skipping) |
| Rust Unused Dependencies (Machete) | ❌ failed | Missing tool: 'cargo-machete'. Guidance: install with `cargo install cargo-machete --locked` (required by tier 'protected-branch'; the gate fails closed instead of skipping) |
| Rust MSRV Audit | ✅ success | Passed in 880.813186ms |
| Shell Script Lint (ShellCheck) | ❌ failed | Missing tool: 'shellcheck'. Guidance: install shellcheck (e.g. `apt-get install shellcheck` or `brew install shellcheck`) (required by tier 'protected-branch'; the gate fails closed instead of skipping) |
| Markdown Lint (markdownlint-cli2) | ❌ failed | Missing tool: 'markdownlint-cli2'. Guidance: install with `npm install -g markdownlint-cli2` (required by tier 'protected-branch'; the gate fails closed instead of skipping) |
| Skill Validation | ✅ success | Passed in 1.00298546s |
| Skill Evaluations | ✅ success | Passed in 8.806999569s |
| Privacy Check (No emails) | ✅ success | Passed in 60.843057ms |
| Secret Scan | ✅ success | Passed in 124.520904ms |
| GitHub Actions Workflow Validation | ✅ success | Passed in 5.317625974s |
| CI Status Artifact Check | ✅ success | Passed in 181.899µs |

## Overall: **FAILURE**
