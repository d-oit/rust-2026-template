#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use criterion::{Criterion, criterion_group, criterion_main};
use mcp_server_template::{CalcTool, EchoTool, McpServer, ToolRequest};
use serde_json::Value;
use std::hint::black_box;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn bench_mcp_server_dispatch(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let server = Arc::new(McpServer::new());
    rt.block_on(async {
        server.register(EchoTool).await.unwrap();
        server.register(CalcTool).await.unwrap();
    });

    let mut group = c.benchmark_group("mcp_server_dispatch");

    // Single-task dispatch
    group.bench_function("single_task_execute_tool", |b| {
        b.iter(|| {
            rt.block_on(async {
                let request = ToolRequest::new(Value::String("bench".into()));
                let response = server.execute_tool(black_box("echo"), request).await;
                black_box(response).unwrap();
            });
        });
    });

    // High-concurrency execute_tool across 16 parallel Tokio tasks
    group.bench_function("high_concurrency_execute_tool", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut handles = Vec::with_capacity(16);
                for _ in 0..16 {
                    let server_clone = Arc::clone(&server);
                    handles.push(tokio::spawn(async move {
                        let request = ToolRequest::new(Value::String("bench".into()));
                        let response = server_clone.execute_tool("echo", request).await;
                        response.unwrap();
                    }));
                }
                for handle in handles {
                    handle.await.unwrap();
                }
            });
        });
    });

    // Concurrent list_tools alongside dispatches
    group.bench_function("concurrent_list_tools_and_dispatch", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut handles = Vec::with_capacity(16);
                for _ in 0..12 {
                    let server_clone = Arc::clone(&server);
                    handles.push(tokio::spawn(async move {
                        let request = ToolRequest::new(Value::String("bench".into()));
                        let _ = server_clone.execute_tool("echo", request).await;
                    }));
                }
                for _ in 0..4 {
                    let server_clone = Arc::clone(&server);
                    handles.push(tokio::spawn(async move {
                        let _tools = server_clone.list_tools().await;
                    }));
                }
                for handle in handles {
                    handle.await.unwrap();
                }
            });
        });
    });

    // Dynamic registration under load
    group.bench_function("register_under_load", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut handles = Vec::with_capacity(16);
                for _ in 0..12 {
                    let server_clone = Arc::clone(&server);
                    handles.push(tokio::spawn(async move {
                        let request = ToolRequest::new(Value::String("bench".into()));
                        let _ = server_clone.execute_tool("echo", request).await;
                    }));
                }
                for _ in 0..4 {
                    let server_clone = Arc::clone(&server);
                    handles.push(tokio::spawn(async move {
                        let _ = server_clone.register(EchoTool).await;
                    }));
                }
                for handle in handles {
                    handle.await.unwrap();
                }
            });
        });
    });

    group.finish();
}

criterion_group!(benches, bench_mcp_server_dispatch);
criterion_main!(benches);
