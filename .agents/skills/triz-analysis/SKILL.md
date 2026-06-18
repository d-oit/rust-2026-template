---
name: triz-analysis
version: "0.2.10"
description: Run a systematic TRIZ contradiction audit against a Rust codebase, architecture, or workflow to identify hidden trade-offs and innovation opportunities. Use when facing design trade-offs, contradictory requirements, or needing to identify innovation opportunities through systematic contradiction analysis. Triggers: "TRIZ audit", "contradiction analysis", "innovation audit", "trade-off analysis", "hidden trade-offs".
category: analysis
license: MIT
metadata:
  author: d-oit
  version: "0.2.10"
  adapted-from: d-o-hub/github-template-ai-agents
  tags: triz analysis innovation contradiction rust architecture audit
---

# TRIZ Analysis — Rust Edition

Systematic innovation audit for Rust codebases, workspace architectures, and CI/CD pipelines.

## When to Use

- Auditing a Rust workspace for technical contradictions
- Reviewing `scripts/`, CI pipelines, or `Cargo.toml` manifests for hidden trade-offs
- Analyzing an architecture for scalability vs. complexity bottlenecks
- Performing a swarm-based TRIZ audit on a Rust project
- Reviewing `unsafe` boundaries, async vs. sync, or `tokio::spawn` vs. `spawn_blocking` choices

## Input Requirements

- **Target Scope**: The specific directory, pipeline, or framework to analyze
  (e.g., `crates/`, `scripts/`, `.github/workflows/`, `agent-coordination`).
- **Language Focus**: Rust-specific patterns (ownership, async, traits, lifetimes).

## Output Convention

All agent-generated analysis must be written to `analysis/` using:

```text
analysis/triz-<scope>-YYYY-MM-DD.md
```

Example: `analysis/triz-scripts-2026-06-18.md`

## Audit Process

1. **SCOPE SCAN**: Examine the target scope for existing pain points or trade-offs.
2. **CONTRADICTION DISCOVERY**: Identify technical contradictions:
   "Improving [X] in this scope causes [Y] to worsen."
3. **PRINCIPLE MAPPING**: Map discovered contradictions to TRIZ inventive principles.
4. **INNOVATION ROADMAP**: Propose specific Rust-idiomatic changes based on TRIZ resolutions.
5. **REPORT**: Document findings in `analysis/`.

## Core Protocol

### Contradiction-First Approach

```text
1. SCAN the scope
2. IDENTIFY contradiction: "Improving [X] causes [Y] to worsen"
3. CHECK if contradiction is real or apparent
4. RESOLVE using separation principles
5. DOCUMENT findings in analysis/
```

### Ideal Final Result (IFR) for Rust Audits

```text
"The ideal Rust system performs its function with zero unsafe, zero clones, zero contention"

1. What is the ideal outcome for this scope?
2. What prevents reaching it? (current type/ownership/CI constraints)
3. Which constraints are real vs assumed?
4. Could the borrow checker enforce correctness instead of runtime checks?
```

## Contradiction Matrix (Rust-Specific)

| Improving | vs Worsening | → Principle | Rust Pattern |
|---|---|---|---|
| Throughput | vs Latency | #7, #13, #17 | Batching, parking_lot::RwLock, lock-free channels |
| API ergonomics | vs Binary size | #1, #15 | Feature flags, `#[cfg]`, trait-based DI |
| Compile time | vs Runtime perf | #13, #35 | LTO off, `cargo-bloat` profile tuning |
| Safety | vs Performance | #12, #4 | `unsafe` minimization, scoped SIMD |
| Genericity | vs Compile time | #2, #6 | Trait specialization, sealed traits |
| Async surface | vs Blocking cost | #1, #15 | `spawn_blocking`, `tokio::task::yield_now` |
| Build matrix | vs CI minutes | #1, #9 | `cargo-hack`, workspace inheritance |
| Test coverage | vs Test runtime | #13, #35 | `cargo-nextest` sharding, proptest shrinking |

## Software Contradiction Matrix (General)

| Improving | vs Worsening | → Principle |
|---|---|---|
| Functionality | vs Complexity | #1, #9, #15 |
| Performance | vs Maintainability | #7, #13, #17 |
| Flexibility | vs Simplicity | #6, #2, #3 |
| Security | vs Usability | #12, #4, #17 |
| Scalability | vs Resource Cost | #1, #13, #35 |

## Separation Principles (Rust Mapping)

| Strategy | When | Rust Pattern |
|---|---|---|
| **Time** | Different behavior at stages | Lifecycle generics, `OnceCell`, `Phased init` |
| **Space** | Different behavior in context | Crate features, `cfg` flags, `target_arch` |
| **Condition** | Different by input | Trait objects, sealed traits, `match` |
| **System-level** | New component resolves | Newtype wrapper, sidecar crate, separate binary |

## Integration with Other Skills

- **agent-coordination**: Orchestrate as a swarm sub-task where multiple agents analyze
  different crates or pipelines in parallel.
- **task-decomposition**: Use to break a large workspace audit into smaller scoped TRIZ analyses.
- **anti-ai-slop**: Apply after TRIZ resolutions to keep the rewrite idiomatic and not boilerplate-heavy.

## Quality Checklist

- Target scope clearly defined
- Discovered contradictions explicitly stated
- Recommendations mapped to TRIZ inventive principles
- Rust-idiomatic pattern suggested (not just generic advice)
- Findings saved to `analysis/triz-<scope>-YYYY-MM-DD.md`

## Rationalizations

| Rationalization | Reality |
|---|---|
| "This Rust code compiles, no contradictions exist" | Compilation proves type safety, not design trade-offs. Many crates have hidden coupling or unsafe audit gaps. |
| "TRIZ is too theoretical for Rust" | TRIZ principles map directly to Rust patterns: segmentation = crate split, taking out = extract trait, asymmetry = `PhantomData`. |
| "Just run clippy, that catches everything" | Clippy catches idiomatic violations; TRIZ catches architectural contradictions clippy cannot see. |

## Red Flags

- [ ] Skipping the contradiction statement and jumping straight to solutions
- [ ] Defining contradictions too vaguely to map to specific TRIZ principles
- [ ] Not saving findings to `analysis/` as specified
- [ ] Proposing non-idiomatic Rust (e.g., dynamic dispatch where static would do)

## Reference Files

- `references/principles.md` - All 40 TRIZ principles with software examples
- `references/patterns.md` - Common software contradiction patterns and resolutions
- `references/evolution.md` - TRIZ evolution trends for system design
- [The Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

## Related Skills

- `triz-solver` - Apply TRIZ principles to solve specific problems
- `task-decomposition` - Break audits into scoped sub-tasks
- `goap-agent` - Use TRIZ in Phase 1 (Analyze) of GOAP
- `anti-ai-slop` - Audit rewritten code for boilerplate and over-abstraction
