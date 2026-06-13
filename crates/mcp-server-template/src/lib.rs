//! # MCP Server Template
//!
//! A template crate demonstrating Model Context Protocol (MCP) server integration
//! with dynamic tool registration.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │        MCP Server (rmcp)             │
//! │  ┌─────────────────────────────┐    │
//! │  │      Tool Registry           │    │
//! │  │  (HashMap name → Box<dyn>)  │    │
//! │  └─────────────────────────────┘    │
//! │           │                          │
//! │           ▼                          │
//! │  ┌─────────────────────────────┐    │
//! │  │      Tool Implementations    │    │
//! │  │  (EchoTool, CalcTool, ...)   │    │
//! │  └─────────────────────────────┘    │
//! └─────────────────────────────────────┘
//! ```
//!
//! ## Features
//!
//! - Tool trait with lifecycle hooks
//! - Registry-based tool dispatch
//! - Request/response validation
//! - JSON schema generation

pub mod tool;
pub mod tools;

use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::info;

pub use tool::{Tool, ToolError, ToolRequest, ToolResponse};
pub use tools::{CalcTool, EchoTool};

/// MCP server error types.
#[derive(Debug, Error)]
pub enum ServerError {
    /// Tool not found in registry.
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    /// Tool execution error.
    #[error("Tool error: {0}")]
    Tool(ToolError),

    /// Server initialization error.
    #[error("Server error: {0}")]
    Init(String),
}

impl From<ToolError> for ServerError {
    fn from(e: ToolError) -> Self {
        Self::Tool(e)
    }
}

/// Tool registry for dynamic dispatch.
pub type ToolRegistry = HashMap<&'static str, Arc<dyn Tool>>;

/// MCP server with registered tools.
pub struct McpServer {
    tools: Arc<RwLock<ToolRegistry>>,
}

impl McpServer {
    /// Create a new MCP server instance.
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a tool with the server.
    pub async fn register<T: Tool + 'static>(&self, tool: T) -> Result<(), ServerError> {
        let name = tool.name();
        info!("Registering tool: {}", name);
        self.tools.write().await.insert(name, Arc::new(tool));
        Ok(())
    }

    /// List all registered tool names.
    pub async fn list_tools(&self) -> Vec<String> {
        self.tools
            .read()
            .await
            .keys()
            .map(|s| s.to_string())
            .collect()
    }

    /// Execute a tool by name with the provided request.
    pub async fn execute_tool(
        &self,
        name: &str,
        request: ToolRequest,
    ) -> Result<ToolResponse, ServerError> {
        let tool = {
            let tools = self.tools.read().await;
            tools
                .get(name)
                .ok_or_else(|| ServerError::ToolNotFound(name.to_string()))?
                .clone()
        };

        tool.validate(&request.input)?;
        tool.handle(request).await.map_err(ServerError::Tool)
    }

    /// Initialize all registered tools.
    pub async fn init(&self) -> Result<(), ServerError> {
        let tool_names: Vec<_> = self.list_tools().await;
        for name in &tool_names {
            let tool = {
                let tools = self.tools.read().await;
                tools.get(name.as_str()).cloned().ok_or_else(|| {
                    ServerError::Init(format!("Tool {name} missing during init"))
                })?
            };
            tool.init().await?;
        }
        Ok(())
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[tokio::test]
    async fn test_server_creation() {
        let server = McpServer::new();
        let tools = server.list_tools().await;
        assert!(tools.is_empty());
    }

    #[tokio::test]
    async fn test_register_and_list_tools() {
        let server = McpServer::new();
        server.register(EchoTool).await.unwrap();
        server.register(CalcTool).await.unwrap();

        let tools = server.list_tools().await;
        assert_eq!(tools.len(), 2);
        assert!(tools.contains(&"echo".to_string()));
        assert!(tools.contains(&"calc".to_string()));
    }

    #[tokio::test]
    async fn test_execute_tool_not_found() {
        let server = McpServer::new();
        let request = ToolRequest::new(Value::String("test".to_string()));
        let result = server.execute_tool("nonexistent", request).await;
        assert!(matches!(result, Err(ServerError::ToolNotFound(_))));
    }

    #[tokio::test]
    async fn test_execute_echo_tool() {
        let server = McpServer::new();
        server.register(EchoTool).await.unwrap();

        let request = ToolRequest::new(Value::String("hello world".to_string()));
        let response = server.execute_tool("echo", request).await.unwrap();
        assert_eq!(response.result, Value::String("hello world".to_string()));
    }

    #[tokio::test]
    async fn test_execute_calc_tool_add() {
        let server = McpServer::new();
        server.register(CalcTool).await.unwrap();

        let request = ToolRequest::new(serde_json::json!({"op": "add", "a": 2.0, "b": 3.0}));
        let response = server.execute_tool("calc", request).await.unwrap();
        assert!(response.success);
        assert_eq!(
            response.result,
            Value::Number(serde_json::Number::from_f64(5.0).unwrap())
        );
    }

    #[tokio::test]
    async fn test_tool_init() {
        let server = McpServer::new();
        server.register(EchoTool).await.unwrap();
        server.register(CalcTool).await.unwrap();

        server.init().await.unwrap();
    }

    #[tokio::test]
    async fn test_tool_request_with_metadata() {
        let request = ToolRequest::new(Value::String("test".to_string()))
            .with_metadata(Value::String("meta".to_string()));
        assert!(request.metadata.is_some());
    }

    #[tokio::test]
    async fn test_tool_response_failure() {
        let response = ToolResponse::failure("error message".to_string());
        assert!(!response.success);
        assert_eq!(response.result, Value::String("error message".to_string()));
    }
}
