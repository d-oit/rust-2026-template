//! Tool trait and associated types.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use thiserror::Error;

/// Request to a tool containing JSON input.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
    fn handle(
        &self,
        request: ToolRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResponse, ToolError>> + Send>>;

    /// Called once at server startup for initialization.
    fn init(&self) -> Pin<Box<dyn Future<Output = Result<(), ToolError>> + Send>> {
        Box::pin(async { Ok(()) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_request_deny_unknown_fields() {
        let json = json!({
            "input": "test",
            "unknown_field": "oops"
        });
        let result: Result<ToolRequest, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown field"));
    }

    #[test]
    fn test_tool_response_deny_unknown_fields() {
        let json = json!({
            "result": "test",
            "success": true,
            "unknown_field": "oops"
        });
        let result: Result<ToolResponse, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown field"));
    }
}
