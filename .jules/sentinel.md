## 2026-05-15 - Enforcing Privacy Policy via Quality Gates
**Vulnerability:** Personal Identifiable Information (PII) leakage through email addresses in the codebase (Cargo.toml, READMEs, etc.).
**Learning:** While the project had a `privacy-first` skill and policy, it was only a guideline for AI agents and lacked automated enforcement in the local development workflow or CI.
**Prevention:** Added a Privacy Check step to `scripts/quality-gates.sh` that scans for email patterns (excluding allowed test domains). This provides a fail-fast mechanism for developers to ensure PII is not committed, aligning the codebase with its stated privacy goals.
