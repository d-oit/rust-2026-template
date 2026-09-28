#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use checkpoint_template::{CheckpointConfig, CheckpointHeader};
use example_registry_pattern::{EchoHandler, Registry, ReverseHandler};
use mcp_server_template::tool::{Tool, ToolRequest};
use mcp_server_template::tools::{CalcTool, EchoTool};
use sample_app::{Config, process_items};

#[test]
fn test_sample_app_default_config_snapshot() {
    let config = Config::default();
    insta::assert_yaml_snapshot!(config);
}

#[test]
fn test_sample_app_process_items_snapshot() {
    let items = process_items(5, 100).expect("process_items should succeed");
    assert_eq!(items.len(), 5);
    insta::assert_yaml_snapshot!(items);
}

#[test]
fn test_checkpoint_header_and_config_snapshot() {
    let header = CheckpointHeader::default();
    let config = CheckpointConfig::default();
    assert_eq!(header.version, 1);
    assert_eq!(config.max_app_name_len, 256);
    insta::assert_yaml_snapshot!("checkpoint_header_default", header);
    insta::assert_yaml_snapshot!("checkpoint_config_default", config);
}

#[tokio::test]
async fn test_mcp_tools_behaviour_snapshot() {
    let calc = CalcTool;
    let request = ToolRequest::new(serde_json::json!({
        "op": "add",
        "a": 42.0,
        "b": 58.0
    }));
    let response = calc
        .handle(request)
        .await
        .expect("calc tool execution failed");
    insta::assert_yaml_snapshot!("mcp_calc_response", response);

    let echo = EchoTool;
    let request = ToolRequest::new(serde_json::json!({
        "message": "hello snapshot harness"
    }));
    let response = echo
        .handle(request)
        .await
        .expect("echo tool execution failed");
    insta::assert_yaml_snapshot!("mcp_echo_response", response);
}

#[test]
fn test_registry_dispatch_behaviour_snapshot() {
    let mut registry = Registry::default();
    registry.register("echo", Box::new(EchoHandler));
    registry.register("reverse", Box::new(ReverseHandler));

    let echo_res = registry
        .dispatch("echo", "hello world")
        .expect("echo dispatch failed");
    let reverse_res = registry
        .dispatch("reverse", "hello world")
        .expect("reverse dispatch failed");

    assert_eq!(echo_res, "hello world");
    assert_eq!(reverse_res, "dlrow olleh");

    let dispatch_results = vec![("echo", echo_res), ("reverse", reverse_res)];
    insta::assert_yaml_snapshot!(dispatch_results);
}
