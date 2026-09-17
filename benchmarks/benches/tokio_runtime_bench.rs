#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

//! Tokio runtime regression harness: execution-strategy, fairness, contention and
//! backpressure workloads.
//!
//! Scope split (see `.agents/skills/tokio-performance/SKILL.md`):
//! - deterministic invariants live in `tests/tokio_runtime_behavior.rs`; they assert
//!   capacity, ordering and deadlock-freedom, never wall-clock thresholds, so they
//!   cannot flake on a shared runner;
//! - this file is informational telemetry. Each group runs a short *instrumented*
//!   sampling pass that reports tail latencies (`/p50`, `/p95`, `/p99`, `/max`), then
//!   a clean criterion measurement. Percentiles are printed in bencher format so
//!   `scripts/parse_bench.py` records them in `benchmarks/events/**` unchanged.
//!
//! Run locally: `cargo bench -p benchmarks --bench tokio_runtime -- --quick`

use criterion::{Criterion, criterion_group, criterion_main};
use mcp_server_template::{CalcTool, EchoTool, McpServer, ToolRequest};
use serde_json::Value;
use std::hint::black_box;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::runtime::{Builder, Runtime};
use tokio::sync::{Semaphore, mpsc};

/// Concurrent work units per fan-out measurement.
const FANOUT: usize = 64;
/// Iterations for one short bounded CPU unit (~tens of microseconds).
const CPU_ITERS: u64 = 20_000;
/// Messages pushed through the service harness.
const MESSAGES: usize = 1_024;
/// Iterations of the repeatedly-ready loop.
const READY_LOOP_ITERS: u64 = 2_000_000;
/// Sampling rounds per latency report.
const SAMPLE_ROUNDS: usize = 100;
/// Sampling rounds for the multi-millisecond ready-loop variants.
const READY_SAMPLE_ROUNDS: usize = 20;
/// Units per fan-out measurement (kept high enough to expose queueing).
const FANOUT_UNITS: usize = 4_096;
/// Permit count for the bounded fan-out measurement.
const FANOUT_CONCURRENCY: usize = 64;
/// Dispatches per registry measurement.
const DISPATCHES: usize = 2_048;
/// Concurrent registrations during the registry measurement.
const WRITES: usize = 64;

/// Short bounded CPU work: deterministic integer mixing, no syscalls.
fn burn(iterations: u64) -> u64 {
    let mut acc = 0x9E37_79B9_7F4A_7C15_u64;
    for i in 0..iterations {
        acc = acc.rotate_left(7) ^ i.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        acc = acc.wrapping_add(acc >> 13);
    }
    acc
}

fn multi_thread_runtime() -> Runtime {
    Builder::new_multi_thread().enable_all().build().unwrap()
}

fn current_thread_runtime() -> Runtime {
    Builder::new_current_thread().enable_all().build().unwrap()
}

fn echo_request() -> ToolRequest {
    ToolRequest::new(Value::String("bench".to_string()))
}

/// Emits percentile rows in criterion's bencher format.
///
/// `scripts/parse_bench.py` matches `test <name> ... bench: <n> ns/iter`, so each
/// percentile becomes its own entry in the JSONL event stream. Integer arithmetic
/// only.
fn report_latencies(name: &str, samples: &mut [Duration]) {
    if samples.is_empty() {
        return;
    }
    samples.sort_unstable();
    let last = samples.len().saturating_sub(1);
    let pick = |numer: usize| samples[last * numer / 100].as_nanos();
    for (label, numer) in [("p50", 50), ("p95", 95), ("p99", 99), ("max", 100)] {
        println!(
            "test {name}/{label} ... bench: {} ns/iter (+/- 0)",
            pick(numer)
        );
    }
}

// ─── CPU execution strategy ────────────────────────────────────────────────────

async fn cpu_fanout_direct() -> u64 {
    let mut units = Vec::with_capacity(FANOUT);
    for _ in 0..FANOUT {
        units.push(tokio::spawn(async { burn(CPU_ITERS) }));
    }
    let mut total = 0_u64;
    for unit in units {
        total = total.wrapping_add(unit.await.unwrap());
    }
    total
}

async fn cpu_fanout_blocking() -> u64 {
    let mut units = Vec::with_capacity(FANOUT);
    for _ in 0..FANOUT {
        units.push(tokio::task::spawn_blocking(|| burn(CPU_ITERS)));
    }
    let mut total = 0_u64;
    for unit in units {
        total = total.wrapping_add(unit.await.unwrap());
    }
    total
}

async fn cpu_fanout_direct_latencies(rounds: usize) -> Vec<Duration> {
    let mut samples = Vec::with_capacity(FANOUT * rounds);
    for _ in 0..rounds {
        let mut units = Vec::with_capacity(FANOUT);
        for _ in 0..FANOUT {
            units.push(tokio::spawn(async {
                let started = Instant::now();
                black_box(burn(CPU_ITERS));
                started.elapsed()
            }));
        }
        for unit in units {
            samples.push(unit.await.unwrap());
        }
    }
    samples
}

async fn cpu_fanout_blocking_latencies(rounds: usize) -> Vec<Duration> {
    let mut samples = Vec::with_capacity(FANOUT * rounds);
    for _ in 0..rounds {
        let mut units = Vec::with_capacity(FANOUT);
        for _ in 0..FANOUT {
            units.push(tokio::task::spawn_blocking(|| {
                let started = Instant::now();
                black_box(burn(CPU_ITERS));
                started.elapsed()
            }));
        }
        for unit in units {
            samples.push(unit.await.unwrap());
        }
    }
    samples
}

/// Samples one short CPU unit per round, executed directly on a worker or handed to
/// the blocking pool, so the handoff cost shows up in the tail report.
async fn cpu_single_latencies(rounds: usize, via_blocking: bool) -> Vec<Duration> {
    let mut samples = Vec::with_capacity(rounds);
    for _ in 0..rounds {
        let started = Instant::now();
        if via_blocking {
            black_box(
                tokio::task::spawn_blocking(|| burn(CPU_ITERS))
                    .await
                    .unwrap(),
            );
        } else {
            black_box(burn(CPU_ITERS));
        }
        samples.push(started.elapsed());
    }
    samples
}

/// Direct execution on Tokio workers versus `spawn_blocking` for the same unit.
fn bench_cpu_execution_strategy(c: &mut Criterion) {
    let rt = multi_thread_runtime();
    let mut group = c.benchmark_group("tokio_cpu");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("single_unit_direct", |b| {
        b.iter(|| black_box(burn(CPU_ITERS)));
    });

    group.bench_function("single_unit_spawn_blocking", |b| {
        b.iter(|| {
            rt.block_on(async {
                black_box(
                    tokio::task::spawn_blocking(|| burn(CPU_ITERS))
                        .await
                        .unwrap(),
                )
            })
        });
    });

    group.bench_function("fanout_direct_on_workers", |b| {
        b.iter(|| rt.block_on(async { black_box(cpu_fanout_direct().await) }));
    });

    group.bench_function("fanout_via_spawn_blocking", |b| {
        b.iter(|| rt.block_on(async { black_box(cpu_fanout_blocking().await) }));
    });

    let mut direct_unit = rt.block_on(cpu_single_latencies(SAMPLE_ROUNDS, false));
    report_latencies("tokio_cpu/single_unit_direct", &mut direct_unit);
    let mut blocking_unit = rt.block_on(cpu_single_latencies(SAMPLE_ROUNDS, true));
    report_latencies("tokio_cpu/single_unit_spawn_blocking", &mut blocking_unit);
    let mut direct = rt.block_on(cpu_fanout_direct_latencies(SAMPLE_ROUNDS));
    report_latencies("tokio_cpu/fanout_direct", &mut direct);
    let mut blocking = rt.block_on(cpu_fanout_blocking_latencies(SAMPLE_ROUNDS));
    report_latencies("tokio_cpu/fanout_spawn_blocking", &mut blocking);
    group.finish();
}

// ─── Cooperative yielding ──────────────────────────────────────────────────────

async fn ready_loop(iterations: u64, yield_every: u64) -> u64 {
    let mut acc = 0_u64;
    for i in 0..iterations {
        if yield_every != 0 && i % yield_every == 0 {
            tokio::task::yield_now().await;
        }
        acc = acc.wrapping_add(i);
    }
    acc
}

/// Cost of `yield_now()` inside a repeatedly-ready loop.
fn bench_cooperative_yield(c: &mut Criterion) {
    let rt = current_thread_runtime();
    let mut group = c.benchmark_group("tokio_fairness");
    group.sample_size(20);

    group.bench_function("ready_loop_without_yield", |b| {
        b.iter(|| rt.block_on(async { black_box(ready_loop(READY_LOOP_ITERS, 0).await) }));
    });

    group.bench_function("ready_loop_yield_every_256", |b| {
        b.iter(|| rt.block_on(async { black_box(ready_loop(READY_LOOP_ITERS, 256).await) }));
    });

    let mut without_yield = rt.block_on(ready_loop_latencies(READY_SAMPLE_ROUNDS, 0));
    report_latencies(
        "tokio_fairness/ready_loop_without_yield",
        &mut without_yield,
    );
    let mut with_yield = rt.block_on(ready_loop_latencies(READY_SAMPLE_ROUNDS, 256));
    report_latencies("tokio_fairness/ready_loop_yield_every_256", &mut with_yield);
    group.finish();
}

/// Samples whole-loop durations for both variants; per-yield cost is the difference.
async fn ready_loop_latencies(rounds: usize, yield_every: u64) -> Vec<Duration> {
    let mut samples = Vec::with_capacity(rounds);
    for _ in 0..rounds {
        let started = Instant::now();
        black_box(ready_loop(READY_LOOP_ITERS, yield_every).await);
        samples.push(started.elapsed());
    }
    samples
}

// ─── Fan-out and backpressure ─────────────────────────────────────────────────

async fn fanout_bounded(units: usize, concurrency: usize) -> u64 {
    let gate = Arc::new(Semaphore::new(concurrency));
    let mut handles = Vec::with_capacity(units);
    for _ in 0..units {
        let permit = Arc::clone(&gate).acquire_owned().await.unwrap();
        handles.push(tokio::spawn(async move {
            tokio::task::yield_now().await;
            drop(permit);
        }));
    }
    for handle in handles {
        handle.await.unwrap();
    }
    units as u64
}

async fn fanout_unbounded(units: usize) -> u64 {
    let mut handles = Vec::with_capacity(units);
    for _ in 0..units {
        handles.push(tokio::spawn(async {
            tokio::task::yield_now().await;
        }));
    }
    for handle in handles {
        handle.await.unwrap();
    }
    units as u64
}

async fn fanout_bounded_latencies(rounds: usize) -> Vec<Duration> {
    let mut samples = Vec::with_capacity(FANOUT_UNITS * rounds);
    for _ in 0..rounds {
        let gate = Arc::new(Semaphore::new(FANOUT_CONCURRENCY));
        let mut handles = Vec::with_capacity(FANOUT_UNITS);
        for _ in 0..FANOUT_UNITS {
            let permit = Arc::clone(&gate).acquire_owned().await.unwrap();
            handles.push(tokio::spawn(async move {
                let started = Instant::now();
                tokio::task::yield_now().await;
                let elapsed = started.elapsed();
                drop(permit);
                elapsed
            }));
        }
        for handle in handles {
            samples.push(handle.await.unwrap());
        }
    }
    samples
}

async fn fanout_unbounded_latencies(rounds: usize) -> Vec<Duration> {
    let mut samples = Vec::with_capacity(FANOUT_UNITS * rounds);
    for _ in 0..rounds {
        let mut handles = Vec::with_capacity(FANOUT_UNITS);
        for _ in 0..FANOUT_UNITS {
            handles.push(tokio::spawn(async {
                let started = Instant::now();
                tokio::task::yield_now().await;
                started.elapsed()
            }));
        }
        for handle in handles {
            samples.push(handle.await.unwrap());
        }
    }
    samples
}

/// Bounded (semaphore) versus unbounded `tokio::spawn` under heavy fan-out.
fn bench_fanout_backpressure(c: &mut Criterion) {
    let rt = multi_thread_runtime();
    let mut group = c.benchmark_group("tokio_fanout");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("bounded_semaphore_64", |b| {
        b.iter(|| {
            rt.block_on(async { black_box(fanout_bounded(FANOUT_UNITS, FANOUT_CONCURRENCY).await) })
        });
    });

    group.bench_function("unbounded_spawn", |b| {
        b.iter(|| rt.block_on(async { black_box(fanout_unbounded(FANOUT_UNITS).await) }));
    });

    let mut bounded = rt.block_on(fanout_bounded_latencies(SAMPLE_ROUNDS));
    report_latencies("tokio_fanout/bounded", &mut bounded);
    let mut unbounded = rt.block_on(fanout_unbounded_latencies(SAMPLE_ROUNDS));
    report_latencies("tokio_fanout/unbounded", &mut unbounded);
    group.finish();
}

// ─── Registry contention ──────────────────────────────────────────────────────

fn bench_registry_contention(c: &mut Criterion) {
    let rt = multi_thread_runtime();
    let server = Arc::new(McpServer::new());
    rt.block_on(async {
        server.register(EchoTool).await.unwrap();
        server.register(CalcTool).await.unwrap();
    });

    let mut group = c.benchmark_group("tokio_registry");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("read_only_dispatch", |b| {
        b.iter(|| {
            rt.block_on(async {
                for _ in 0..DISPATCHES {
                    black_box(server.execute_tool("echo", echo_request()).await.unwrap());
                }
            });
        });
    });

    group.bench_function("dispatch_with_concurrent_writer", |b| {
        b.iter(|| {
            rt.block_on(async {
                let writer = tokio::spawn({
                    let server = Arc::clone(&server);
                    async move {
                        for _ in 0..WRITES {
                            server.register(EchoTool).await.unwrap();
                        }
                    }
                });
                for _ in 0..DISPATCHES {
                    black_box(server.execute_tool("echo", echo_request()).await.unwrap());
                }
                writer.await.unwrap();
            });
        });
    });

    let mut samples = Vec::with_capacity(DISPATCHES);
    rt.block_on(async {
        let writer = tokio::spawn({
            let server = Arc::clone(&server);
            async move {
                for _ in 0..WRITES {
                    server.register(EchoTool).await.unwrap();
                }
            }
        });
        for _ in 0..DISPATCHES {
            let started = Instant::now();
            black_box(server.execute_tool("echo", echo_request()).await.unwrap());
            samples.push(started.elapsed());
        }
        writer.await.unwrap();
    });
    report_latencies("tokio_registry/contended_dispatch", &mut samples);
    group.finish();
}

// ─── Actor-style service queue ────────────────────────────────────────────────

async fn service_bounded(capacity: usize, messages: usize) -> u64 {
    let (tx, mut rx) = mpsc::channel::<u64>(capacity);
    let worker = tokio::spawn(async move {
        let mut processed = 0_u64;
        while let Some(value) = rx.recv().await {
            processed = processed.wrapping_add(value);
        }
        processed
    });
    for value in 0..messages as u64 {
        tx.send(value).await.unwrap();
    }
    drop(tx);
    worker.await.unwrap()
}

async fn service_unbounded(messages: usize) -> u64 {
    let (tx, mut rx) = mpsc::unbounded_channel::<u64>();
    let worker = tokio::spawn(async move {
        let mut processed = 0_u64;
        while let Some(value) = rx.recv().await {
            processed = processed.wrapping_add(value);
        }
        processed
    });
    for value in 0..messages as u64 {
        tx.send(value).unwrap();
    }
    drop(tx);
    worker.await.unwrap()
}

async fn service_bounded_latencies(
    capacity: usize,
    messages: usize,
    rounds: usize,
) -> Vec<Duration> {
    let mut samples = Vec::with_capacity(messages * rounds);
    for _ in 0..rounds {
        let (tx, mut rx) = mpsc::channel::<(Instant, u64)>(capacity);
        let worker = tokio::spawn(async move {
            while let Some((enqueued, _value)) = rx.recv().await {
                black_box(enqueued);
            }
        });
        for value in 0..messages as u64 {
            let enqueued = Instant::now();
            tx.send((enqueued, value)).await.unwrap();
            samples.push(enqueued.elapsed());
        }
        drop(tx);
        worker.await.unwrap();
    }
    samples
}

/// Queue depth, throughput and end-to-end latency for a bounded and an unbounded queue.
fn bench_service_queue(c: &mut Criterion) {
    let rt = multi_thread_runtime();
    let mut group = c.benchmark_group("tokio_service");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("bounded_queue_64", |b| {
        b.iter(|| rt.block_on(async { black_box(service_bounded(64, MESSAGES).await) }));
    });

    group.bench_function("unbounded_queue", |b| {
        b.iter(|| rt.block_on(async { black_box(service_unbounded(MESSAGES).await) }));
    });

    let mut bounded = rt.block_on(service_bounded_latencies(64, MESSAGES, SAMPLE_ROUNDS));
    report_latencies("tokio_service/bounded_queue_64", &mut bounded);
    let mut unbounded = rt.block_on(service_unbounded_latencies(MESSAGES, SAMPLE_ROUNDS));
    report_latencies("tokio_service/unbounded_queue", &mut unbounded);
    group.finish();
}

/// Samples enqueue-to-receive latency for the unbounded queue.
async fn service_unbounded_latencies(messages: usize, rounds: usize) -> Vec<Duration> {
    let mut samples = Vec::with_capacity(messages * rounds);
    for _ in 0..rounds {
        let (tx, mut rx) = mpsc::unbounded_channel::<(Instant, u64)>();
        let worker = tokio::spawn(async move {
            while let Some((enqueued, _value)) = rx.recv().await {
                black_box(enqueued);
            }
        });
        for value in 0..messages as u64 {
            let enqueued = Instant::now();
            tx.send((enqueued, value)).unwrap();
            samples.push(enqueued.elapsed());
        }
        drop(tx);
        worker.await.unwrap();
    }
    samples
}

criterion_group!(
    benches,
    bench_cpu_execution_strategy,
    bench_cooperative_yield,
    bench_fanout_backpressure,
    bench_registry_contention,
    bench_service_queue
);
criterion_main!(benches);
