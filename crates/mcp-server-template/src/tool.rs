//! Tool trait and associated types.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// Request to a tool containing JSON input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequest {
    /// The input value for the tool.
    pub input: Value,

    /// Optional metadata for the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl ToolRequest {
    /// Create a new tool request with the given input.
    pub const fn new(input: Value) -> Self {
        Self {
            input,
            metadata: None,
        }
    }

    /// Set metadata on the request.
    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Response from a tool containing JSON result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse {
    /// The output value from the tool.
    pub result: Value,

    /// Whether the tool execution succeeded.
    pub success: bool,
}

impl ToolResponse {
    /// Create a successful response.
    pub const fn success(result: Value) -> Self {
        Self {
            result,
            success: true,
        }
    }

    /// Create a failed response.
    #[allow(clippy::missing_const_for_fn)]
    pub fn failure(error: String) -> Self {
        Self {
            result: serde_json::Value::String(error),
            success: false,
        }
    }
}

/// Tool error types.
#[derive(Debug, Error)]
pub enum ToolError {
    /// Invalid input provided to the tool.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Execution failed.
    #[error("Execution error: {0}")]
    Execution(String),

    /// Validation failed.
    #[error("Validation error: {0}")]
    Validation(String),
}

/// Tool validation errors for JSON schema violations.
#[derive(Debug, Error)]
pub enum ValidationError {
    /// Missing required field.
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Invalid field value.
    #[error("Invalid value for field {0}: {1}")]
    InvalidValue(String, String),
}

/// Implement this trait for each MCP tool.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Returns the tool's unique name.
    fn name(&self) -> &'static str;

    /// Returns the tool's description for discovery.
    fn description(&self) -> &'static str;

    /// Validate the input before execution.
    fn validate(&self, input: &Value) -> Result<(), ToolError> {
        let _ = input;
        Ok(())
    }

    /// Handle the tool execution.
    async fn handle(&self, request: ToolRequest) -> Result<ToolResponse, ToolError>;

    /// Called once at server startup for initialization.
    async fn init(&self) -> Result<(), ToolError> {
        Ok(())
    }
}
