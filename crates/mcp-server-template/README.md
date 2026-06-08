# MCP Server Template

A template crate demonstrating Model Context Protocol (MCP) server integration with dynamic tool registration.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
mcp-server-template = { path = "../mcp-server-template" }
```

## Basic Setup

```rust
use mcp_server_template::{McpServer, EchoTool, CalcTool};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = McpServer::new();

    // Register tools
    server.register(EchoTool).await?;
    server.register(CalcTool).await?;

    // List available tools
    let tools = server.list_tools().await;
    println!("Available tools: {:?}", tools);

    // Execute a tool
    let request = serde_json::json!({"message": "hello"});
    let response = server.execute_tool("echo", request.into()).await?;
    println!("Result: {:?}", response.result);

    Ok(())
}
```

## Creating Custom Tools

Implement the `Tool` trait:

```rust
use mcp_server_template::{Tool, ToolRequest, ToolResponse, ToolError};
use async_trait::async_trait;

pub struct MyTool;

#[async_trait]
impl Tool for MyTool {
    fn name(&self) -> &'static str {
        "my_tool"
    }

    fn description(&self) -> &'static str {
        "Does something useful"
    }

    async fn handle(&self, request: ToolRequest) -> Result<ToolResponse, ToolError> {
        // Your implementation here
        Ok(ToolResponse::success(serde_json::json!({"status": "ok"})))
    }
}
```

## Architecture

- `McpServer` - Main server type with tool registry
- `Tool` trait - Define tools with name/description/handle
- `ToolRequest/ToolResponse` - JSON-based I/O types
- `ToolError` - Error handling for tools