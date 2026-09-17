#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

//! Tokio pressure telemetry: fan-out, registry contention and service queues.
//!
//! Informational only — deterministic invariants for the same behaviours live in
//! `tests/tokio_runtime_behavior.rs` (no wall-clock thresholds). Each group runs a
//! short instrumented sampling pass that reports `/p50`, `/p95`, `/p99` and `/max`,
//! then a clean criterion measurement.
//!
//! Execution-strategy and fairness workloads live in `tokio_runtime_bench.rs`;
//! shared helpers in `tokio_common`.
//!
//! Run locally: `cargo bench -p benchmarks --bench tokio_runtime_pressure -- --quick`

mod tokio_common;

use crate::tokio_common::{
    DISPATCHES, FANOUT_CONCURRENCY, FANOUT_SAMPLE_ROUNDS, FANOUT_UNITS, MESSAGES, SAMPLE_ROUNDS,
    WRITES, echo_request, multi_thread_runtime, report_latencies,
};
use criterion::{Criterion, criterion_group, criterion_main};
use mcp_server_template::{CalcTool, EchoTool, McpServer};
use std::hint::black_box;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Semaphore, mpsc};

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

    let mut bounded = rt.block_on(fanout_bounded_latencies(FANOUT_SAMPLE_ROUNDS));
    report_latencies("tokio_fanout/bounded", &mut bounded);
    let mut unbounded = rt.block_on(fanout_unbounded_latencies(FANOUT_SAMPLE_ROUNDS));
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

criterion_group!(
    benches,
    bench_fanout_backpressure,
    bench_registry_contention,
    bench_service_queue
);
criterion_main!(benches);
