---
name: tokio-performance
description: "Audit and design Tokio async Rust systems for throughput, latency, fairness, contention, blocking, and bounded concurrency. Use for Tokio runtime design, async performance reviews, task scheduling, spawn/spawn_blocking decisions, locks, channels, I/O, and tail-latency problems."
category: performance
license: MIT
metadata:
  author: d-oit
  version: "0.1.0"
  tags: rust tokio async performance latency throughput fairness contention concurrency
---

# Tokio Performance

Performance guidance for production Rust systems using Tokio. Optimize measured behavior, not async code by reflex.

## Core Rule

**Measure first. Choose the execution model from workload characteristics.**

Do not assume:
- `async` is faster than synchronous code
- `spawn_blocking` is always better for CPU work
- `tokio::sync::Mutex` is preferable to a synchronous lock
- more tasks means more throughput
- yielding everywhere improves performance

## Decision Protocol

1. Identify the workload: I/O-bound, short CPU-bound, long CPU-bound, blocking, bursty, or latency-sensitive.
2. Define the target: throughput, median latency, tail latency, fairness, startup time, or resource usage.
3. Measure before changing scheduling or synchronization.
4. Keep critical sections short and never hold locks across unrelated `.await` points.
5. Bound fan-out and queue growth when downstream capacity is finite.
6. Re-measure under representative load.

## Execution Model

### Direct execution

Use direct execution for small, bounded CPU work when keeping the work on the Tokio worker avoids unnecessary task-pool coordination. Keep poll sections short enough to preserve scheduler fairness.

### `spawn_blocking`

Use `spawn_blocking` for genuinely blocking operations or CPU work that is long enough to interfere with Tokio worker progress. Treat it as a scheduling/isolation mechanism, not a universal performance primitive.

### Dedicated isolation

For sustained or highly specialized CPU workloads, consider a dedicated worker pool/thread or separate runtime when isolation is a real requirement. Do not add multiple runtimes without evidence of contention or isolation needs.

## Scheduling & Fairness

- Avoid long synchronous sections inside async tasks.
- Beware repeatedly-ready loops that can monopolize a worker.
- Use batching when it materially reduces scheduling/coordination overhead.
- Use `tokio::task::yield_now()` deliberately when fairness is required; do not insert it mechanically.
- Optimize tail latency separately from throughput.

## Concurrency & Backpressure

Prefer bounded designs:
- bounded `mpsc` channels
- `Semaphore` for finite downstream capacity
- worker pools for controlled fan-out
- explicit queue limits and timeouts

Avoid unbounded `spawn()` fan-out, especially around network, filesystem, external APIs, or expensive computation.

## Synchronization

Prefer, in order where practical:
1. ownership and message passing
2. immutable/shared data
3. short synchronous locks for data-only critical sections
4. async locks only when waiting while holding the lock is genuinely required

For a registry/cache pattern:

```text
lock -> lookup/clone -> unlock -> await/execute
```

Never perform expensive work or external I/O while holding a shared lock.

## I/O

Async filesystem/network APIs prevent the caller from blocking directly, but they are not zero-cost. Many small operations can create scheduling and coordination overhead.

Prefer:
- batching logically related operations
- reducing needless syscalls
- buffering where appropriate
- measuring filesystem behavior on the target OS

Do not replace correct async I/O with synchronous I/O merely for theoretical speed.

## Observability

When performance matters, capture enough information to distinguish:
- scheduler delay
- task/poll duration
- queue depth and backpressure
- task fan-out
- blocking-pool usage
- lock contention
- I/O latency
- application work time

Use benchmarks and representative load tests. Optimize p95/p99 when tail latency is part of the requirement.

## Review Checklist

- [ ] Workload type is explicitly identified
- [ ] Performance objective is explicit
- [ ] `spawn_blocking` is justified by workload, not used automatically
- [ ] No unbounded task fan-out
- [ ] Queues have deliberate capacity/backpressure
- [ ] Locks are short-lived and not held across unrelated `.await`
- [ ] Hot loops have a fairness strategy
- [ ] I/O operations are not unnecessarily fragmented
- [ ] Performance claims are benchmarked
- [ ] Changes are evaluated for both throughput and tail latency where relevant

## Anti-Patterns

- "Everything CPU-heavy goes to `spawn_blocking`."
- "Everything shared needs `Arc<RwLock<_>>`."
- "Add `yield_now()` to every loop."
- "Spawn one Tokio task per item without a bound."
- "Use a second runtime because Tokio is slow."
- "Benchmark only a happy-path microbenchmark and generalize to production."

## Integration

- Use `triz-analysis` for async-vs-blocking, throughput-vs-latency, and contention contradictions.
- Use `anti-ai-slop` after performance refactors to prevent speculative abstractions.
- Use `build-rust` and the repository quality gates after implementation.

## References

- Tokio documentation: https://tokio.rs/tokio/tutorial
- Rust Performance Book: https://nnethercote.github.io/perf-book/
- Principles for Fast Tokio Applications: https://dial9-rs.github.io/blog/principles-for-fast-tokio-applications/
