# MCP Server Template

> Model Context Protocol (MCP) server integration with dynamic tool registration and dispatch.

## When to use

- Building MCP-compatible servers that expose tools to AI agents
- Creating plugin systems with runtime tool registration
- Systems needing JSON Schema-based input validation for tool calls

## Quick start

```rust,ignore
use mcp_server_template::{McpServer, Tool, ToolRequest, ToolResponse, ToolError};
use async_trait::async_trait;
use serde_json::Value;

struct GreetTool;

#[async_trait]
impl Tool for GreetTool {
    fn name(&self) -> &'static str { "greet" }
    fn description(&self) -> &'static str { "Greets a user by name" }

    async fn handle(&self, request: ToolRequest) -> Result<ToolResponse, ToolError> {
        let name = request.input.as_str().unwrap_or("World");
        Ok(ToolResponse::success(Value::String(format!("Hello, {name}!"))))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = McpServer::new();
    server.register(GreetTool).await?;
    server.init().await?;

    let tools = server.list_tools().await;
    println!("Available tools: {tools:?}");

    let response = server.execute_tool(
        "greet",
        ToolRequest::new(Value::String("Rust".into())),
    ).await?;
    println!("Result: {}", response.result);
    Ok(())
}
```

## Creating Custom Tools

Implement the `Tool` trait:

```rust,ignore
use mcp_server_template::{Tool, ToolRequest, ToolResponse, ToolError};
use async_trait::async_trait;

pub struct MyTool;

#[async_trait]
impl Tool for MyTool {
    fn name(&self) -> &'static str { "my_tool" }
    fn description(&self) -> &'static str { "Does something useful" }

    fn validate(&self, input: &serde_json::Value) -> Result<(), ToolError> {
        // Optional: validate input before handle() runs
        Ok(())
    }

    async fn handle(&self, request: ToolRequest) -> Result<ToolResponse, ToolError> {
        Ok(ToolResponse::success(serde_json::json!({"status": "ok"})))
    }
}
```

## Built-in Tools

| Tool | Description | Input |
|------|-------------|-------|
| `EchoTool` | Returns input as-is | Any JSON value |
| `CalcTool` | Basic arithmetic | Object with `op`, `a`, `b` fields |

## Architecture

- **`McpServer`** — Registry-based tool dispatch with async RwLock
- **`Tool`** trait — `name()`, `description()`, `validate()`, `handle()`, `init()`
- **`ToolRequest`** / **`ToolResponse`** — JSON-based I/O with optional metadata
- **`ToolError`** — Typed errors: `InvalidInput`, `Execution`, `Validation`
