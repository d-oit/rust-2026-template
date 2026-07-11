# Harness Engineering

> Agent = Model + Harness. This document is the harness map for rust-2026-template.
> Based on: https://martinfowler.com/articles/harness-engineering.html

## Mental Model

The harness has two axes:
- **Feedforward (guides):** What to do *before* writing code — context, constraints, conventions
- **Feedback (sensors):** What fires *after* writing code — automated checks that catch violations

And two modes:
- **Computational:** Deterministic (clippy, tests, deny) — always trust the output
- **Inferential:** LLM-based (skill docs, agent context) — direction, not commands

## Feedforward Guides

### Inferential (read before coding)

| Guide | Path | Purpose |
|---|---|---|
| Agent contract | `AGENTS.md` | Root coding conventions, change workflow, quality gates |
| Skills index | `.agents/SKILLS.md` | Available executable task knowledge |
| Harness overview | `HARNESS.md` (this file) | How guides and sensors connect |
| Harness skill | `.agents/skills/harness/SKILL.md` | Sensor response protocol and self-correction |
| Clippy intent | `.clippy.toml` | Linting philosophy and allowed exceptions |
| Dependency rules | `deny.toml` | Crate layering rules (`*-types → *-core → *-adapters → *-cli`) |
| Architecture | `plans/adr/` | Architecture Decision Records |
| Cross-repo context | `.agents/context/shared-conventions.md` | Commit format, branch naming, PR requirements |

### Computational (structural constraints)

| Constraint | File | Enforced by |
|---|---|---|
| Crate layering | `deny.toml` | `cargo deny check` |
| No unsafe code | `Cargo.toml` `[workspace.lints.rust]` | `rustc` |
| Max 500 LOC/file | `AGENTS.md` | Agent self-check |
| Conventional commits | `commitlint.config.cjs` | `commitlint` pre-commit hook |

## Feedback Sensors

### Computational (deterministic — always trust)

| Sensor | Trigger | Config | LLM Fix Hint |
|---|---|---|---|
| `cargo fmt --check` | pre-commit | `.pre-commit-config.yaml` | Run `cargo fmt --all` |
| `cargo clippy -D warnings` | pre-commit + CI | `.clippy.toml`, `.pre-commit-config.yaml` | Fix all warnings; see `.clippy.toml` for allowed exceptions |
| `cargo deny check` | pre-commit + CI | `deny.toml` | Check crate layering diagram in `Cargo.toml` comments |
| `cargo nextest run` | CI (`ci.yml`) | `Cargo.toml` | Fix failing tests before opening PR |
| `cargo mutants` | CI weekly (`mutants.yml`) | `[workspace.metadata.cargo-mutants]` in `Cargo.toml` | If score < threshold, add targeted unit tests |
| `shellcheck` | pre-commit | `.shellcheckrc` | Fix shell script issues at severity=warning |
| `gitleaks` | CI (`security-scan.yml`) | `.gitleaks.toml` | Remove secrets; use env vars or `.env` |
| Architecture fitness | `tests/arch_fitness.rs` | `Cargo.toml` dev-deps | HARNESS VIOLATION message includes fix instructions |
| Snapshot tests | `tests/behaviour_harness.rs` | `Cargo.toml` `insta = "=1.47.2"` | Run `cargo insta review` to approve new baselines |

### Inferential (LLM-based — use for direction)

| Sensor | Path | Purpose |
|---|---|---|
| Codacy quality review | `.codacy.yml`, CI | Code quality suggestions |
| Codecov coverage | `.codecov.yml`, CI | Coverage regression detection |
| AI skill evaluator | `.agents/skills/skill-evaluator/` | Evaluate skill effectiveness |

## Steering Loop

When any sensor fires **repeatedly** (>2 times in one sprint):
1. Identify the root cause category (maintainability / architecture / behaviour)
2. Update the corresponding **feedforward guide** to prevent recurrence
3. If no guide exists, create one in `.agents/skills/` using the `skill-creator` skill
4. Document the update in `CHANGELOG.md`

The steering loop closes the harness: sensors fire → humans and agents update guides → sensors fire less.

## Self-Correction Protocol for Agents

When a computational sensor fires:
1. Read the full error message — it includes a fix hint
2. Identify category: fmt / lint / test / arch / security
3. Apply the minimal fix (do not refactor unrelated code)
4. Re-run the specific sensor: `cargo clippy`, `cargo test`, etc.
5. Only commit when the sensor is green
6. Write a metrics event to `.agents/events/YYYY/MM/DD/` per the `metrics-reporter` skill

## Agent-Optimised Error Output

The `scripts/harness-check.sh` wrapper runs each sensor and emits structured error output with `HARNESS VIOLATION` prefix and agent-parseable fix hints. Use it for richer feedback than raw sensor output:

```bash
bash scripts/harness-check.sh <fmt|clippy|deny|test|arch|all>
```

## Lint Policy

Workspace-level lints (`Cargo.toml`) set the **default** for all crates.
Individual crates may relax specific lints in their own `[lints.clippy]`
section when there is a documented reason. Never use `#[allow(...)]`
attributes in source code — this is enforced by `allow_attributes = "deny"`.

See `scripts/harness-check.sh` for the full sensor → hint mapping.
