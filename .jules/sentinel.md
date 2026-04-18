## 2026-05-15 - Enforcing Privacy Policy via Quality Gates
**Vulnerability:** Personal Identifiable Information (PII) leakage through email addresses in the codebase (Cargo.toml, READMEs, etc.).
**Learning:** While the project had a `privacy-first` skill and policy, it was only a guideline for AI agents and lacked automated enforcement in the local development workflow or CI.
**Prevention:** Added a Privacy Check step to `scripts/quality-gates.sh` that scans for email patterns (excluding allowed test domains). This provides a fail-fast mechanism for developers to ensure PII is not committed, aligning the codebase with its stated privacy goals.

## 2026-05-15 - Workspace-Wide Memory Safety and Strict Schema Validation
**Vulnerability:** Potential for memory safety bypass and configuration injection.
**Learning:** Using `#![deny(unsafe_code)]` allows submodules to override the restriction with `#![allow(unsafe_code)]`, which can lead to undetected memory safety issues. Additionally, allowing unknown fields in configuration structs can lead to "configuration injection" or mass assignment vulnerabilities.
**Prevention:** 1) Upgrade to `#![forbid(unsafe_code)]` at the workspace and crate roots to prevent any local overrides. 2) Enforce `#[serde(deny_unknown_fields)]` on configuration structs to ensure only strictly defined schemas are accepted. 3) Use `serde_json::from_reader` with `BufReader` and `file.take(limit)` for secure, memory-efficient streaming of configuration data.

## 2025-04-07 - Secure Configuration Loading Pattern
**Vulnerability:** Denial of Service (DoS) via memory exhaustion when loading untrusted or oversized configuration files.
**Learning:** Standard `std::fs::read_to_string` is susceptible to memory exhaustion if the input file is large or is a special device (like `/dev/zero`) that provides an infinite stream. Simple `metadata.len()` checks can be bypassed by TOCTOU or special files reporting zero size.
**Prevention:** Use a multi-layered approach: 1) Open the file first, 2) Verify it is a regular file using `file.metadata()?.is_file()`, 3) Enforce a strict size limit using `file.take(limit).read_to_string(&mut contents)`. This ensures predictable memory usage and prevents blocking on non-regular files.

## 2026-04-11 - Mitigating Supply Chain Attacks via Dependency Rollback and Banning
**Vulnerability:** Supply chain compromise via malicious crate versions (e.g., `serde_json` v1.0.149) that introduce non-standard, suspicious dependencies (`zmij`, `serde_core`).
**Learning:** Malicious artifacts can masquerade as standard updates. Relying on "latest" versions without checking for dependency graph anomalies (like the replacement of `ryu` with `zmij`) can leave applications vulnerable to remote code execution or data exfiltration.
**Prevention:** 1) Pin core dependencies to verified safe, exact versions in `Cargo.toml` (e.g., `=1.0.x`). 2) Use `cargo tree` to audit for unexpected or non-standard dependencies. 3) Configure `deny.toml` with an explicit `[bans]` list to prevent suspicious crates from being introduced even as transitive dependencies.

## 2026-04-18 - Hardening Configuration and Preventing Log Injection
**Vulnerability:** Log injection via un-sanitized configuration fields and Denial of Service (DoS) via memory exhaustion from excessively large numeric parameters.
**Learning:** Deserializing data with `serde` only ensures schema correctness, not value safety. String fields can contain control characters like newlines, which can corrupt log output or be used for log forging. Numeric fields can be valid for the type (e.g., `usize`) but dangerously large for application logic (e.g., pre-allocating a `Vec` with billions of items).
**Prevention:** 1) Sanitize string fields after loading by filtering out control characters (`is_control()`). 2) Define and enforce reasonable upper bounds (`MAX_ALLOWED_ITEMS`) for numeric fields used in resource allocations immediately after loading the configuration.
