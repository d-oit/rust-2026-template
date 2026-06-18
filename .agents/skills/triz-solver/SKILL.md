---
name: triz-solver
version: "0.2.10"
description: Systematic problem-solving using TRIZ (Theory of Inventive Problem Solving) principles adapted for software engineering. Use when stuck on complex problems, facing technical contradictions, optimizing system design, or seeking innovative solutions beyond trial-and-error. Prevents solving the wrong problem correctly. Triggers: "TRIZ solve", "inventive solution", "contradiction resolution", "stuck on design", "TRIZ principle".
category: innovation-problem-solving
license: MIT
metadata:
  author: d-oit
  version: "0.2.10"
  adapted-from: d-o-hub/github-template-ai-agents
  tags: triz solver innovation contradiction rust design problem-solving
---

# TRIZ Problem Solver — Rust Edition

Systematic innovation methodology for Rust engineering problems.

## When to Use

- Complex Rust problems with apparent trade-offs
- System design contradictions (improving X worsens Y)
- Architecture refactoring decisions in a workspace
- Seeking innovative solutions beyond incremental improvement
- Agent instruction optimization
- Stuck on borrow-checker, lifetime, or trait-bound conflicts

## Core Protocol

### Contradiction-First Approach

```text
1. STATE the problem in one sentence
2. IDENTIFY contradiction: "Improving [X] causes [Y] to worsen"
3. CHECK if contradiction is real or apparent
4. RESOLVE using separation principles
5. VALIDATE no new contradictions introduced
```

### Ideal Final Result (IFR)

```text
"The ideal Rust solution delivers all benefits with zero cost, zero unsafe, zero clones"

1. What is the ideal outcome? (no implementation details)
2. What prevents reaching it? (Rust constraints: ownership, lifetimes, traits)
3. Which constraints are real vs assumed?
4. Can the type system enforce correctness without runtime checks?
```

## 40 Inventive Principles (Software Edition)

### Top 10 for Rust

| # | Principle | Rust Analogy | When to Apply |
|---|---|---|---|
| 1 | **Segmentation** | Split crate, module, function | Monolithic code, complex functions |
| 2 | **Taking out** | Extract trait, separate `mod` | Mixed responsibilities in one type |
| 3 | **Local quality** | Domain-specific types (`Url`, `UserId`) | Generic `String` everywhere |
| 4 | **Asymmetry** | Feature-gated impls, sealed traits | One-size-fits-all antipattern |
| 5 | **Merging** | Batch operations, reduce round-trips | Excessive function calls, I/O |
| 6 | **Universality** | Shared trait, polymorphism | Duplicate implementations |
| 7 | **Nesting** | `Cow`, lifetime hierarchies | Flat complex types |
| 12 | **Inversion** | Pull-based deps, IoC, callbacks | Current push approach failing |
| 13 | **Dynamics** | Runtime config, feature flags, hot-reload | Static rigid configurations |
| 17 | **Another dimension** | Add newtype, wrapper, sidecar | Impasse at current abstraction level |

### Extended Reference

See `references/principles.md` for all 40 principles with software examples.

## Contradiction Matrix (Software)

```text
Improving            vs Worsening              → Principle
──────────────────────────────────────────────────────────────
Functionality        vs Complexity             → #1, #9, #15
Performance          vs Maintainability        → #7, #13, #17
Flexibility          vs Simplicity             → #6, #2, #3
Security             vs Usability              → #12, #4, #17
Scalability          vs Resource Cost          → #1, #13, #35
```

### Rust-Specific Additions

```text
Compile time         vs Runtime perf           → #13, #35
API ergonomics       vs Binary size            → #1, #15
Async surface        vs Blocking cost          → #1, #15
Genericity           vs Compile time           → #2, #6
```

## Decision Workflow

### Step 1: Frame Problem

```markdown
Problem: [One sentence]
Current approach: [What you're doing]
Stuck because: [Why it doesn't work]
Rust constraints hit: [Ownership / Lifetime / Trait bounds / Async / Unsafe]
```

### Step 2: Identify Contradiction

```markdown
Improving: [Parameter that needs improvement]
Worsens: [Parameter that degrades]
Type: [Technical / Physical / Inherent]
Rust-specific: [Is the borrow checker the bottleneck?]
```

### Step 3: Apply Resolution

Choose separation strategy:

- **Time**: Different behavior at different stages
  - Rust: `OnceCell`, `Phased` init, staged builder
- **Space**: Different behavior in different contexts
  - Rust: feature flags, `#[cfg]`, `target_arch`
- **Condition**: Different behavior based on input type
  - Rust: trait objects, sealed traits, `match`
- **System-level**: Add component that resolves both
  - Rust: newtype wrapper, sidecar crate, separate binary

### Step 4: Validate

```markdown
✓ Original problem solved
✓ No new contradictions introduced
✓ Solution approaches IFR (minimal complexity added)
✓ `cargo clippy --all-targets --all-features -- -D warnings` passes
✓ `cargo nextest run --workspace` passes
✓ No `unsafe` without `// SAFETY:` rationale
```

## Examples

### Example 1: Compile Time vs Runtime Performance

```text
Problem: Heavy generics in core crate cause slow dev builds
Contradiction: Genericity (improve) vs Compile time (worsen)
Apply: #2 Taking out → Extract hot-loop into runtime-dispatched `dyn Trait` crate
Result: Core crate compiles fast, hot loop dispatched at runtime
```

### Example 2: API Ergonomics vs Binary Size

```text
Problem: Convenience methods bloat the binary
Contradiction: API ergonomics (improve) vs Binary size (worsen)
Apply: #1 Segmentation → Feature-gate convenience methods, default off
Result: Users opt in to convenience, base binary stays small
```

### Example 3: Async Surface vs Blocking Cost

```text
Problem: Exposing async requires every caller to .await
Contradiction: Async surface (improve) vs Blocking cost (worsen)
Apply: #12 Inversion → Provide blocking wrapper that calls `tokio::runtime::Handle::block_on`
Result: Both sync and async callers served
```

### Example 4: Security vs Usability (Auth)

```text
Problem: Strong auth improves security but hurts UX
Contradiction: Security (improve) vs Usability (worsen)
Apply: #12 Inversion → Risk-based auth (challenge only suspicious sessions)
Result: Both sync and async callers served
```

### Example 4: Agent Instruction Optimization

```text
Problem: More instructions improve reliability but consume context
Contradiction: Completeness (improve) vs Token limits (worsen)
Apply: #4 Asymmetry → Heavy knowledge in `references/`, load lazily
Result: Reliability maintained, context efficiency improved
```

### Example 5: Lifetime vs Extensibility

```text
Problem: Adding a borrow makes the trait less flexible for downstream impls
Contradiction: Borrow safety (improve) vs Extensibility (worsen)
Apply: #7 Nesting → Use `Cow<'_, T>` or lifetime-generic type parameter
Result: Borrow enforced where needed, flexible where not
```

## Integration with Other Skills

- **task-decomposition**: Use TRIZ to identify contradictions before decomposing
- **agent-coordination**: Apply IFR to coordination strategy selection
- **iterative-refinement**: Use contradiction analysis to guide refinement cycles
- **triz-analysis**: Audit first, then solve
- **anti-ai-slop**: Validate the TRIZ solution isn't boilerplate-heavy

## Quality Checklist

- Contradiction explicitly stated before solution
- IFR defined before implementation
- Solution verified against contradiction matrix
- No new contradictions introduced
- Approaches ideal state (minimal added complexity, idiomatic Rust)
- `cargo clippy` and `cargo nextest` pass

## Rationalizations

| Rationalization | Reality |
|---|---|
| "I already know the solution, no need for TRIZ analysis" | Skipping contradiction analysis often means solving the wrong problem correctly, wasting implementation effort. |
| "This is just a minor tweak, not a real problem" | Minor tweaks can introduce new contradictions; validate that no new trade-offs are created. |
| "Just add `unsafe` to bypass the borrow checker" | `unsafe` is the cost; TRIZ avoids paying it. Re-examine the design first. |

## Red Flags

- [ ] Proposing a solution without explicitly stating the contradiction first
- [ ] Skipping IFR definition and jumping to implementation details
- [ ] Introducing new contradictions without validating the solution
- [ ] Using `unsafe` to bypass the borrow checker instead of redesigning
- [ ] Adding `unwrap()`, `clone()`, or `Box<dyn Trait>` as the first solution

## Reference Files

- `references/principles.md` - All 40 TRIZ principles with software examples
- `references/patterns.md` - Common software contradiction patterns and resolutions
- `references/evolution.md` - TRIZ evolution trends for system design
- [The Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rustonomicon](https://doc.rust-lang.org/nomicon/)

## Related Skills

- `triz-analysis` - Audit first, then solve
- `task-decomposition` - Break complex problems into sub-tasks
- `goap-agent` - Use TRIZ in Phase 1 (Analyze) of GOAP
- `anti-ai-slop` - Audit rewritten code for boilerplate
