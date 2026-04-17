---
name: anti-ai-slop
description: >
  Apply this skill when auditing, writing, or reviewing Rust code to avoid generic
  "AI slop" patterns: boilerplate structs with no purpose, copy-paste error handling,
  meaningless variable names, over-engineered abstractions, or hollow documentation.
  Triggers: "this looks AI-generated", "too much boilerplate", "meaningless names",
  "over-abstracted", "audit my code", "humanize this", "why is this so verbose",
  "remove unnecessary abstractions", "fix docs", "anti-pattern review".
license: MIT
metadata:
  author: d-oit
  version: "1.0"
  adapted-from: d-o-hub/github-template-ai-agents
  tags: rust code-quality anti-patterns review audit
---

# Anti-AI-Slop Skill — Rust Edition

AI tools generate recognizable Rust monoculture. This skill audits and fixes it.

---

## How to Use

1. **Audit mode** — Share code. Run through checklists below. Name every pattern.
2. **Creation mode** — Read "What to do instead" first, then write code avoiding patterns.
3. **Spot-fix mode** — Point to one element. Diagnose it, rewrite it, explain the fix.

Always **name the problem** before fixing it.

---

## Part 1 — Code Structure Anti-Patterns

| Pattern | What it looks like | Why it's slop |
|---|---|---|
| **Wrapper struct for no reason** | `struct MyError(String)` with no extra behavior | Just use `anyhow::Error` or the inner type directly |
| **Over-engineered trait hierarchy** | `trait Processor<T: Handler<U>>` for one impl | One trait, one impl = delete the trait |
| **`mod utils`** | `mod utils { pub fn helper(...) }` | No concept. Scatter into real modules |
| **`mod common`** | Grab-bag module used by everything | Symptom of no domain model |
| **Phantom generics** | `struct Foo<T> { _marker: PhantomData<T> }` with no bounds | If T does nothing, remove the generic |
| **Re-export sprawl** | `pub use inner::*` chains 3 levels deep | Forces users to trace invisible paths |
| **Builder for 2 fields** | `FooBuilder::new().name(x).value(y).build()` | Just use `Foo { name: x, value: y }` |

### What to Do Instead

- **Model the domain.** Name types after what they represent, not their role in the code.
- **Flat module tree.** One level deep is almost always enough.
- **Direct construction.** Struct literals over builders unless >5 optional fields or invariants.
- **Concrete before abstract.** Start with one function. Extract trait only when you have 2+ impls.

---

## Part 2 — Error Handling Anti-Patterns

| Pattern | What it looks like | Why it's slop |
|---|---|---|
| **`.unwrap()` in library code** | `value.parse().unwrap()` | Panics in calling code without context |
| **`.expect("should not happen")** | everywhere | If it can happen, handle it. If not, prove it. |
| **`Box<dyn Error>` in all signatures** | `fn run() -> Result<(), Box<dyn Error>>` | Erases error type, forces dynamic dispatch |
| **`map_err(\|_\| "failed")** | Error message that says nothing | Include the input, the operation, what failed |
| **`match err { _ => return Err(...) }`** | Manual error forwarding | Use `?` operator |
| **Custom error type per function** | `ParseConfigError`, `LoadConfigError`, `ReadConfigError` | One `ConfigError` with variants |

### What to Do Instead

- Use `thiserror` for library errors, `anyhow` for application errors.
- Every error message must answer: what was I doing, what input failed, what to do next.
- One error enum per module, not per function.

---

## Part 3 — Documentation Anti-Patterns

| Pattern | What it looks like | Why it's slop |
|---|---|---|
| **Restates the name** | `/// Creates a new Foo.` for `fn new_foo()` | Zero information gain |
| **`# Examples` with trivial code** | Example that just calls `new()` | Show a real use case or delete the section |
| **Passive voice everywhere** | "This function is used to..." | "Creates", "Returns", "Validates" — be direct |
| **`TODO: implement`** | Left in committed code | Either implement it or open an issue |
| **`// Safety: this is safe`** | In `unsafe` blocks | Explain **why** the invariants hold, not that they do |

### What to Do Instead

- First line: one sentence, imperative, what it does for the caller.
- `# Errors` section: enumerate what `Err` variants are returned and when.
- `# Panics` section: document every panic condition explicitly.
- `# Safety` section in `unsafe`: state the invariants required, not just "safe".

---

## Part 4 — Naming Anti-Patterns

| Pattern | What it looks like | Fix |
|---|---|---|
| `data`, `info`, `result` as field names | `struct Foo { data: Vec<u8> }` | Name the content: `payload`, `bytes`, `records` |
| `Manager`, `Handler`, `Processor` types | `struct DataProcessor` | What does it process? `InvoiceParser`, `LogNormalizer` |
| Single-letter generics beyond `T` | `fn foo<A, B, C, D>` | Name the role: `Key`, `Value`, `Input`, `Output` |
| `is_true`, `has_data` booleans | `if obj.is_true()` | Name the predicate: `is_authenticated`, `has_pending_items` |
| `temp`, `tmp`, `val`, `x` in non-trivial scope | 20-line function with `let tmp = ...` | Name what it holds |

---

## Part 5 — Audit Workflow

1. **Scan** — Check all four pattern lists. Flag every match by name.
2. **Score severity:**
   - 🔴 **Structural** — Wrong abstraction level. Requires refactor.
   - 🟡 **Surface** — Wrong name, wrong error message, missing doc section.
   - 🟢 **Cosmetic** — Minor polish.
3. **Prioritize** — Fix structural first. Don't rename fields on a wrong abstraction.
4. **Rewrite** — Provide the specific replacement, not generic advice.
5. **Explain the why** — Name the Rust principle behind each fix.

---

## Positive Doctrine

- **Domain names > technical roles.** Types are nouns from the problem space.
- **Errors are first-class.** Design error types with as much care as success types.
- **Docs are contracts.** If it can fail or panic, document it. No exceptions.
- **Flat is better than deep.** One layer of abstraction does more than three thin ones.
- **Zero-cost means earned.** Don't add generics for flexibility you won't use.
