#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

//! Tokio execution-strategy and fairness telemetry.
//!
//! Scope split (see `.agents/skills/tokio-performance/SKILL.md`):
//! - deterministic invariants live in `tests/tokio_runtime_behavior.rs`; they assert
//!   capacity, ordering and deadlock-freedom, never wall-clock thresholds;
//! - this target is informational telemetry. Each group runs a short *instrumented*
//!   sampling pass that reports tail latencies (`/p50`, `/p95`, `/p99`, `/max`), then
//!   a clean criterion measurement.
//!
//! Pressure workloads (fan-out, registry contention, service queue) live in
//! `tokio_runtime_pressure_bench.rs`; shared helpers in `tokio_common`.
//!
//! Run locally: `cargo bench -p benchmarks --bench tokio_runtime -- --quick`

mod tokio_common;

use crate::tokio_common::{
    CPU_ITERS, FANOUT, READY_LOOP_ITERS, READY_SAMPLE_ROUNDS, SAMPLE_ROUNDS, burn,
    current_thread_runtime, multi_thread_runtime, report_latencies,
};
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::time::{Duration, Instant};

// ─── CPU execution strategy ────────────────────────────────────────────────────

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

criterion_group!(
    benches,
    bench_cpu_execution_strategy,
    bench_cooperative_yield
);
criterion_main!(benches);
