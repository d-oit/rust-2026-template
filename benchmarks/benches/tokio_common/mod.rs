#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    missing_docs
)]

//! Shared helpers for the Tokio runtime harness bench targets.
//!
//! Both `tokio_runtime_bench` (execution strategy, fairness) and
//! `tokio_runtime_pressure_bench` (fan-out, registry, service queue) include this
//! module so constants and reporting stay defined once.

use mcp_server_template::ToolRequest;
use serde_json::Value;
use std::time::Duration;
use tokio::runtime::{Builder, Runtime};

/// Concurrent work units per CPU fan-out measurement.
pub const FANOUT: usize = 64;
/// Iterations for one short bounded CPU unit (~tens of microseconds).
pub const CPU_ITERS: u64 = 20_000;
/// Messages pushed through the service harness.
pub const MESSAGES: usize = 1_024;
/// Iterations of the repeatedly-ready loop.
pub const READY_LOOP_ITERS: u64 = 2_000_000;
/// Sampling rounds per latency report.
pub const SAMPLE_ROUNDS: usize = 100;
/// Sampling rounds for the multi-millisecond ready-loop variants.
pub const READY_SAMPLE_ROUNDS: usize = 20;
/// Sampling rounds for fan-out variants, which spawn `FANOUT_UNITS` tasks per round.
pub const FANOUT_SAMPLE_ROUNDS: usize = 20;
/// Units per fan-out measurement (kept high enough to expose queueing).
pub const FANOUT_UNITS: usize = 4_096;
/// Permit count for the bounded fan-out measurement.
pub const FANOUT_CONCURRENCY: usize = 64;
/// Dispatches per registry measurement.
pub const DISPATCHES: usize = 2_048;
/// Concurrent registrations during the registry measurement.
pub const WRITES: usize = 64;

/// Short bounded CPU work: deterministic integer mixing, no syscalls.
pub fn burn(iterations: u64) -> u64 {
    let mut acc = 0x9E37_79B9_7F4A_7C15_u64;
    for i in 0..iterations {
        acc = acc.rotate_left(7) ^ i.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        acc = acc.wrapping_add(acc >> 13);
    }
    acc
}

pub fn multi_thread_runtime() -> Runtime {
    Builder::new_multi_thread().enable_all().build().unwrap()
}

pub fn current_thread_runtime() -> Runtime {
    Builder::new_current_thread().enable_all().build().unwrap()
}

pub fn echo_request() -> ToolRequest {
    ToolRequest::new(Value::String("bench".to_string()))
}

/// Emits percentile rows in criterion's bencher format.
///
/// `scripts/parse_bench.py` matches `test <name> ... bench: <n> ns/iter`, so each
/// percentile becomes its own entry in the JSONL event stream. Criterion 0.8's own
/// bencher output omits the benchmark id, so these named rows are what the event
/// pipeline records. Integer arithmetic only.
pub fn report_latencies(name: &str, samples: &mut [Duration]) {
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
