# Quality Gate Run Summary

**Timestamp:** 2026-10-17T14:07:55Z
**Commit:** 10cccd918062e4593b0486d9f9508cb7c4a60a3a
**Branch:** jules-9691265984351109438-3066c80c

| Check | Status | Details |
|---|---|---|
| LOC Limits | ✅ success | Passed in 32.699412ms |
| Rust Format | ✅ success | Passed in 1.049284026s |
| Rust Clippy | ✅ success | Passed in 3.891018347s |
| Rust Build | ✅ success | Passed in 5.335274946s |
| Rust Tests | ✅ success | Passed in 5.617040578s |
| Rust Doc Tests | ✅ success | Passed in 478.464793ms |
| Rust Security Audit | ❌ failed | Missing tool: 'cargo-audit'. Guidance: install with `cargo install cargo-audit --locked` (required by tier 'protected-branch'; the gate fails closed instead of skipping) |
| Rust Dependency Policy (Deny) | ❌ failed | Missing tool: 'cargo-deny'. Guidance: install with `cargo install cargo-deny --locked` (required by tier 'protected-branch'; the gate fails closed instead of skipping) |
| Rust Unused Dependencies (Machete) | ❌ failed | Missing tool: 'cargo-machete'. Guidance: install with `cargo install cargo-machete --locked` (required by tier 'protected-branch'; the gate fails closed instead of skipping) |
| Rust MSRV Audit | ✅ success | Passed in 2.248386s |
| Shell Script Lint (ShellCheck) | ❌ failed | Missing tool: 'shellcheck'. Guidance: install shellcheck (e.g. `apt-get install shellcheck` or `brew install shellcheck`) (required by tier 'protected-branch'; the gate fails closed instead of skipping) |
| Markdown Lint (markdownlint-cli2) | ❌ failed | Missing tool: 'markdownlint-cli2'. Guidance: install with `npm install -g markdownlint-cli2` (required by tier 'protected-branch'; the gate fails closed instead of skipping) |
| Skill Validation | ✅ success | Passed in 1.281180675s |
| Skill Evaluations | ✅ success | Passed in 11.006897606s |
| Privacy Check (No emails) | ✅ success | Passed in 70.678312ms |
| Secret Scan | ✅ success | Passed in 138.183484ms |
| GitHub Actions Workflow Validation | ✅ success | Passed in 6.782052752s |
| CI Status Artifact Check | ✅ success | Passed in 223.326µs |

## Overall: **FAILURE**
