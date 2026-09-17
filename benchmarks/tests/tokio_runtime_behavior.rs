#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

//! Deterministic behaviour assertions for the Tokio runtime harness.
//!
//! Every assertion here is an invariant — channel capacity, task ordering, thread
//! identity, permit bounds or deadlock-freedom — that holds regardless of machine
//! speed. No wall-clock performance threshold is asserted anywhere: absolute timings
//! live in `benches/tokio_runtime_bench.rs` as informational telemetry.

use mcp_server_template::{CalcTool, EchoTool, McpServer, ToolRequest};
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::{Semaphore, mpsc};

fn current_thread_runtime() -> Runtime {
    Builder::new_current_thread().enable_all().build().unwrap()
}

fn multi_thread_runtime() -> Runtime {
    Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
}

/// A send that cannot complete yet must stay pending: capacity is an invariant, not
/// a timing property, so asserting `Err(Elapsed)` cannot flake.
#[test]
fn bounded_channel_blocks_producer_at_capacity() {
    let rt = current_thread_runtime();
    rt.block_on(async {
        let (tx, mut rx) = mpsc::channel::<u32>(2);
        tx.send(1).await.unwrap();
        tx.send(2).await.unwrap();

        assert!(
            tokio::time::timeout(Duration::from_millis(25), tx.send(3))
                .await
                .is_err(),
            "send completed while the bounded channel was at capacity"
        );

        assert_eq!(rx.recv().await, Some(1));
        tokio::time::timeout(Duration::from_secs(5), tx.send(3))
            .await
            .expect("send still stalled after capacity was freed")
            .unwrap();
    });
}

#[test]
fn unbounded_channel_never_blocks_producer() {
    let rt = current_thread_runtime();
    rt.block_on(async {
        let (tx, mut rx) = mpsc::unbounded_channel::<u32>();
        for value in 0..10_000_u32 {
            tx.send(value).unwrap();
        }
        drop(tx);
        let mut received = 0_u32;
        while rx.recv().await.is_some() {
            received = received.wrapping_add(1);
        }
        assert_eq!(received, 10_000);
    });
}

/// A repeatedly-ready task on a single worker holds the thread: the spawned peer
/// cannot be polled until the loop awaits. This is the starvation the skill warns
/// about, expressed without any timing assumption.
#[test]
fn ready_loop_starves_peer_on_single_worker() {
    let rt = current_thread_runtime();
    rt.block_on(async {
        let peer_ran = Arc::new(AtomicBool::new(false));
        let peer = tokio::spawn({
            let peer_ran = Arc::clone(&peer_ran);
            async move {
                peer_ran.store(true, Ordering::SeqCst);
            }
        });

        let mut acc = 0_u64;
        for i in 0..2_000_000_u64 {
            acc = acc.wrapping_add(i);
        }
        std::hint::black_box(acc);

        assert!(
            !peer_ran.load(Ordering::SeqCst),
            "peer ran while a repeatedly-ready task never yielded"
        );

        peer.await.unwrap();
        assert!(peer_ran.load(Ordering::SeqCst));
    });
}

#[test]
fn yielding_lets_peer_progress_during_ready_loop() {
    let rt = current_thread_runtime();
    rt.block_on(async {
        let peer_ran = Arc::new(AtomicBool::new(false));
        let peer = tokio::spawn({
            let peer_ran = Arc::clone(&peer_ran);
            async move {
                peer_ran.store(true, Ordering::SeqCst);
            }
        });

        let mut acc = 0_u64;
        let mut observed_at = None;
        for i in 0..2_000_000_u64 {
            if i % 256 == 0 {
                tokio::task::yield_now().await;
                if observed_at.is_none() && peer_ran.load(Ordering::SeqCst) {
                    observed_at = Some(i);
                }
            }
            acc = acc.wrapping_add(i);
        }
        std::hint::black_box(acc);

        assert!(
            observed_at.is_some_and(|i| i < 2_000_000),
            "peer task never progressed during a yielding ready loop"
        );
        peer.await.unwrap();
    });
}

/// `spawn_blocking` must hand work to a thread that is not the runtime worker.
#[test]
fn spawn_blocking_runs_off_the_worker_thread() {
    let rt = current_thread_runtime();
    rt.block_on(async {
        let worker = std::thread::current().id();
        let blocking = tokio::task::spawn_blocking(std::thread::current)
            .await
            .unwrap();
        assert_ne!(
            worker,
            blocking.id(),
            "spawn_blocking executed on the runtime worker thread"
        );
    });
}

/// Blocking work parked in the blocking pool must not prevent async progress on a
/// single-worker runtime; a regression here deadlocks and fails the timeout.
#[test]
fn blocking_work_does_not_stall_async_progress() {
    let rt = current_thread_runtime();
    rt.block_on(async {
        let (signal, wait) = std::sync::mpsc::channel::<()>();
        let blocking = tokio::task::spawn_blocking(move || {
            wait.recv().expect("signalled by the async side");
            42_u32
        });

        let joined = tokio::time::timeout(Duration::from_secs(5), async {
            signal.send(()).expect("blocking task is still listening");
            blocking.await.expect("blocking join")
        })
        .await;

        assert_eq!(
            joined.expect("blocking work starved async progress"),
            42_u32
        );
    });
}

/// Bounded fan-out: in-flight work can never exceed the permit count, and every unit
/// still completes.
#[test]
fn bounded_fanout_never_exceeds_permit_count() {
    const PERMITS: usize = 4;
    const UNITS: usize = 64;

    let rt = current_thread_runtime();
    rt.block_on(async {
        let gate = Arc::new(Semaphore::new(PERMITS));
        let in_flight = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::with_capacity(UNITS);

        for _ in 0..UNITS {
            let permit = Arc::clone(&gate).acquire_owned().await.unwrap();
            let in_flight = Arc::clone(&in_flight);
            let peak = Arc::clone(&peak);
            handles.push(tokio::spawn(async move {
                let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(now, Ordering::SeqCst);
                tokio::task::yield_now().await;
                in_flight.fetch_sub(1, Ordering::SeqCst);
                drop(permit);
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert!(
            peak.load(Ordering::SeqCst) <= PERMITS,
            "in-flight work exceeded the permit count"
        );
        assert_eq!(in_flight.load(Ordering::SeqCst), 0);
    });
}

/// The copy-on-write registry must not lose concurrent registrations.
#[test]
fn concurrent_registrations_are_not_lost() {
    let rt = multi_thread_runtime();
    rt.block_on(async {
        let server = Arc::new(McpServer::new());
        let mut handles = Vec::new();

        for _ in 0..4 {
            let echo_server = Arc::clone(&server);
            handles.push(tokio::spawn(async move {
                echo_server.register(EchoTool).await.unwrap();
            }));
            let calc_server = Arc::clone(&server);
            handles.push(tokio::spawn(async move {
                calc_server.register(CalcTool).await.unwrap();
            }));
        }
        for handle in handles {
            handle.await.unwrap();
        }

        let names = server.list_tools().await;
        assert!(
            names.contains(&"echo".to_string()),
            "echo registration lost"
        );
        assert!(
            names.contains(&"calc".to_string()),
            "calc registration lost"
        );

        let response = server
            .execute_tool(
                "echo",
                ToolRequest::new(Value::String("harness".to_string())),
            )
            .await
            .unwrap();
        assert!(response.success);
    });
}
