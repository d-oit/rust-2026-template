---
name: tokio-performance
description: "Audit and design Tokio async Rust systems for throughput, latency, fairness, contention, blocking, and bounded concurrency. Use for Tokio runtime design, async performance reviews, task scheduling, spawn/spawn_blocking decisions, locks, channels, I/O, and tail-latency problems."
category: performance
license: MIT
metadata:
  author: d-oit
  version: "0.2.0"
  tags: rust tokio async performance latency throughput fairness contention concurrency
---

# Tokio Performance

Performance guidance for production Rust systems using Tokio. Optimize measured behavior from workload characteristics, not async code by reflex.

## Core Rule

**Measure first. Derive execution models, concurrency bounds, and lock strategies from workload characteristics.**

Do not assume:
- `async` is faster than synchronous code
- `spawn_blocking` is always better for CPU work
- `tokio::sync::Mutex` or `RwLock` is preferable to a synchronous lock or atomic/snapshot pattern
- more tasks or more runtimes mean more throughput
- `yield_now()` everywhere improves fairness

## Workload Decision Matrix

| Workload Type | Characteristics | Recommended Execution Model | Key Considerations |
|---|---|---|---|
| **Short Bounded CPU** | Fast, non-blocking calculations (< 10–100 µs), memory parsing, small serializations | **Direct Execution on Tokio Worker** | Offloading to `spawn_blocking` introduces thread context switches and queue overhead that exceed the work duration. |
| **Genuinely Blocking** | Synchronous disk I/O, blocking C/FFI calls, legacy blocking SDKs, blocking DB drivers | `spawn_blocking` or Dedicated Thread Pool | Prevents blocking the Tokio worker thread event loop. |
| **Long or Sustained CPU** | Image processing, heavy cryptography, compression, large payload parsing (> 100 µs - ms+) | `spawn_blocking` or **Dedicated Worker Pool** | Protects Tokio worker threads from starvation. For sustained heavy CPU, prefer a dedicated Rayon/thread pool to avoid exhausting Tokio's blocking pool. |
| **Batchable Operations** | High-rate small I/O operations, frequent channel sends, small database updates | **Batching & Micro-buffering** | Consolidates syscalls, locks, and channel synchronization overhead. |
| **Isolated Subsystems** | Multi-tenant latency isolation, distinct thread priority requirements | **Workload Isolation** | Isolate via bounded semaphores or separate thread pools before considering a separate Tokio runtime. |

## Correctness vs. Performance Heuristics

Distinguish non-negotiable correctness rules from workload-driven performance heuristics:

### Correctness Constraints (Mandatory)
- **Never hold lock guards across `.await` points or during I/O:** Holding a std/tokio lock guard across `.await` leads to deadlocks, starvation, or sending non-`Send` guard types across thread boundaries.
- **Never perform blocking synchronous syscalls directly on Tokio worker threads:** Blocking a worker thread starves all other tasks multiplexed on that worker thread.

### Performance Heuristics (Workload-Driven)
- **`spawn_blocking` vs. Direct Execution:** Balance context switch/queue overhead against worker thread yield time (~10–100 µs threshold).
- **Lock Type Selection:** `std::sync::Mutex` (short data-only operations) vs. `tokio::sync::Mutex` (critical sections that MUST hold state across `.await` or await turn access).
- **Cooperative Yielding:** Use `yield_now()` only when measurements or loop structure demonstrate worker thread starvation in repeatedly-ready loops.

## Decision Protocol

1. Identify the workload: I/O-bound, short CPU-bound, long CPU-bound, blocking sync API, bursty, or latency-sensitive.
2. Define the target: throughput, median latency, tail latency (p95/p99), fairness, or resource limits.
3. Distinguish correctness constraints from performance heuristics.
4. Bound fan-out and queue growth by default when downstream capacity is finite.
5. Keep lock scopes minimal and never hold locks across unrelated `.await` calls.
6. Measure under representative load before and after changes.

## Scheduling & Fairness

- **Avoid long synchronous sections** inside async tasks.
- **Repeatedly-ready worker loops:** Tasks that compute or poll continuously without hitting an I/O yield point can starve other tasks assigned to the same worker thread. Evaluate for scheduler starvation and tail-latency impact.
- **`tokio::task::yield_now()`:** Use `yield_now()` deliberately only when measurements or tight loop structure demonstrate starvation. Do not insert it mechanically. Prefer Tokio's automatic cooperative budget (`tokio::task::consume_budget`) or event-driven channel backpressure.
- **Batching:** Use batching (e.g., `buffer_unordered`, batch channel receives) when it reduces scheduling, syscall, or task coordination overhead.

## Bounded Concurrency & Backpressure

Always default to bounded designs for network, filesystem, database, external API calls, or worker queues:
- **Bounded Channels:** Prefer `tokio::sync::mpsc::channel(capacity)` over unbounded channels to enforce upstream backpressure.
- **Bounded Fan-out:** Use `tokio::sync::Semaphore`, `futures::stream::StreamExt::buffer_unordered`, or fixed worker pools rather than unbounded `tokio::spawn` loops.
- **Timeouts & Shedding:** Combine queue capacity limits with `tokio::time::timeout` and explicit load-shedding to prevent cascading failures under heavy load.

## Synchronization & Lock Lifetimes

Prefer synchronization mechanisms in this order:
1. **Ownership and Message Passing:** Move data via bounded channels (`mpsc`, `oneshot`).
2. **Immutable Snapshots:** Use `Arc<T>`, `ArcSwap`, or copy-on-write (`Cow`) for read-heavy shared state.
3. **Short Synchronous Locks:** Use `std::sync::Mutex` or `parking_lot::Mutex` for extremely short data-only updates (e.g., updating a counter or inserting into a HashMap). Release the lock immediately before any `.await`.
4. **Async Locks:** Use `tokio::sync::Mutex` or `tokio::sync::RwLock` ONLY when access across `.await` points is genuinely required or when waiting for lock acquisition needs to be async non-blocking.

Pattern for state lookup and async execution:

```text
lock -> lookup / clone snapshot -> drop lock -> await external work
```

## Anti-Patterns & Counter-Examples

### Counter-Example 1: When `spawn_blocking` is NOT the Answer

**Problem:** Wrapping short CPU tasks (e.g. quick string parsing, fast JSON field extraction, small hash calculation) in `spawn_blocking`.

```rust
// BAD: Cargo-culting spawn_blocking for a 2-microsecond JSON parse
let user: User = tokio::task::spawn_blocking(move || {
    serde_json::from_str(&payload)
}).await??;

// GOOD: Direct execution on Tokio worker thread avoids thread pool queue overhead
let user: User = serde_json::from_str(&payload)?;
```

### Counter-Example 2: When `yield_now` is NOT the Answer

**Problem:** Inserting `yield_now()` manually inside tight processing loops instead of structuring work with backpressure or batching.

```rust
// BAD: Blindly scattering yield_now in every iteration
for item in items {
    process_item(item);
    tokio::task::yield_now().await;
}

// GOOD: Process in batches or leverage channel backpressure and Tokio budget
for chunk in items.chunks(100) {
    process_batch(chunk);
    // Tokio automatically manages cooperative budget for async operations,
    // or explicit yield only if heavy CPU batching risks worker thread starvation.
}
```

### Counter-Example 3: When `RwLock` is NOT the Answer

**Problem:** Wrapping read-heavy config or state in `Arc<tokio::sync::RwLock<Config>>` causing reader lock contention and async overhead.

```rust
// BAD: Heavy async RwLock overhead for read-heavy state lookups
let config = state.read().await;
let val = config.get("key");

// GOOD: Immutable snapshot or atomic swap (e.g., ArcSwap or Arc<Config>)
let config = state.load(); // ArcSwap clone-free atomic read
let val = config.get("key");
```

### Counter-Example 4: When a Second Tokio Runtime is NOT the Answer

**Problem:** Creating a multi-runtime setup (e.g. `Builder::new_multi_thread().build()`) to "isolate" background work.

```rust
// BAD: Spinning up a complete second Tokio runtime inside an async context
let secondary_rt = tokio::runtime::Runtime::new().unwrap();
secondary_rt.block_on(async { ... });

// GOOD: Use bounded tasks, Semaphore, or dedicated std::thread pool for isolated work
let permit = semaphore.acquire_owned().await?;
tokio::spawn(async move {
    let _permit = permit;
    perform_background_work().await;
});
```

### Counter-Example 5: Lock Held Across `.await` Point

**Problem:** Holding a `std::sync::MutexGuard` or `tokio::sync::MutexGuard` across an external `.await` point.

```rust
// BAD: Lock held across async HTTP call blocks all other accessors
let mut guard = state.lock().unwrap();
let data = fetch_remote_data(&guard.url).await?; // Deadlock/contention risk!
guard.last_result = data;

// GOOD: Read state, drop lock, await I/O, re-acquire lock for update
let url = {
    let guard = state.lock().unwrap();
    guard.url.clone()
};
let data = fetch_remote_data(&url).await?;
{
    let mut guard = state.lock().unwrap();
    guard.last_result = data;
}
```

## Observability

Capture metrics to distinguish execution phase behavior:
- task scheduling delay
- task poll duration
- queue depth and channel capacity
- `spawn_blocking` pool usage
- lock wait time and contention
- I/O latency vs application work time

## Review Checklist

- [ ] Workload type (short CPU, blocking sync, long CPU, batchable, isolated) is explicitly identified.
- [ ] Execution model (`direct`, `spawn_blocking`, dedicated pool) is chosen based on workload characteristics.
- [ ] Correctness constraints (lock across await, blocking on worker thread) are respected.
- [ ] No unbounded task fan-out (`Semaphore`, `buffer_unordered`, or bounded channel used).
- [ ] Channels have deliberate capacity limits and backpressure.
- [ ] Locks are data-only, short-lived, and never held across `.await` calls or external I/O.
- [ ] Repeatedly-ready loops are evaluated for fairness and scheduler starvation without cargo-culting `yield_now()`.
- [ ] Claims of async performance improvements are verified by benchmarks under representative load.

## Integration & Cross-References

- **`AGENTS.md`**: Canonical project contract for coding agents.
- **`agents-docs/conventions.md`**: Project-wide coding conventions and invariants.
- **`triz-analysis`**: Use `.agents/skills/triz-analysis/SKILL.md` for evaluating throughput vs. latency, contention, and async vs. blocking trade-offs using TRIZ inventive principles.
- **`anti-ai-slop`**: Use after performance refactors to keep code simple and readable.

## References

- Tokio documentation: https://tokio.rs/tokio/tutorial
- Principles for Fast Tokio Applications: https://dial9-rs.github.io/blog/principles-for-fast-tokio-applications/
- Rust Performance Book: https://nnethercote.github.io/perf-book/
