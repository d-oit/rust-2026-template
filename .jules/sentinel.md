## 2026-05-15 - Enforcing Privacy Policy via Quality Gates
**Vulnerability:** Personal Identifiable Information (PII) leakage through email addresses in the codebase (Cargo.toml, READMEs, etc.).
**Learning:** While the project had a `privacy-first` skill and policy, it was only a guideline for AI agents and lacked automated enforcement in the local development workflow or CI.
**Prevention:** Added a Privacy Check step to `scripts/quality-gates.sh` that scans for email patterns (excluding allowed test domains). This provides a fail-fast mechanism for developers to ensure PII is not committed, aligning the codebase with its stated privacy goals.

## 2025-04-07 - Secure Configuration Loading Pattern
**Vulnerability:** Denial of Service (DoS) via memory exhaustion when loading untrusted or oversized configuration files.
**Learning:** Standard `std::fs::read_to_string` is susceptible to memory exhaustion if the input file is large or is a special device (like `/dev/zero`) that provides an infinite stream. Simple `metadata.len()` checks can be bypassed by TOCTOU or special files reporting zero size.
**Prevention:** Use a multi-layered approach: 1) Open the file first, 2) Verify it is a regular file using `file.metadata()?.is_file()`, 3) Enforce a strict size limit using `file.take(limit).read_to_string(&mut contents)`. This ensures predictable memory usage and prevents blocking on non-regular files.
