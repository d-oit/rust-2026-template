//! Example tool implementations.

use super::tool::{Tool, ToolError, ToolRequest, ToolResponse};
use async_trait::async_trait;
use serde_json::Value;

/// Tool that echoes the input back.
pub struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &'static str {
        "echo"
    }

    fn description(&self) -> &'static str {
        "Returns the input exactly as received"
    }

    async fn handle(&self, request: ToolRequest) -> Result<ToolResponse, ToolError> {
        Ok(ToolResponse::success(request.input))
    }
}

/// Calculator tool supporting basic arithmetic.
pub struct CalcTool;

#[async_trait]
impl Tool for CalcTool {
    fn name(&self) -> &'static str {
        "calc"
    }

    fn description(&self) -> &'static str {
        "Performs basic arithmetic: add, sub, mul, div"
    }

    fn validate(&self, input: &Value) -> Result<(), ToolError> {
        if !input.is_object() {
            return Err(ToolError::InvalidInput("Expected object".to_string()));
        }
        let obj = input.as_object().unwrap();
        if !obj.contains_key("op") || !obj.contains_key("a") || !obj.contains_key("b") {
            return Err(ToolError::InvalidInput("Missing op, a, or b".to_string()));
        }
        Ok(())
    }

    async fn handle(&self, request: ToolRequest) -> Result<ToolResponse, ToolError> {
        let obj = request
            .input
            .as_object()
            .ok_or_else(|| ToolError::Execution("Invalid request format".to_string()))?;

        let op = obj
            .get("op")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("Missing op field".to_string()))?;

        let a = obj
            .get("a")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::InvalidInput("Missing a field".to_string()))?;

        let b = obj
            .get("b")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::InvalidInput("Missing b field".to_string()))?;

        let result = match op {
            "add" => a + b,
            "sub" => a - b,
            "mul" => a * b,
            "div" => {
                if b == 0.0 {
                    return Err(ToolError::Execution("Division by zero".to_string()));
                }
                a / b
            }
            _ => return Err(ToolError::InvalidInput(format!("Unknown operation: {op}"))),
        };

        Ok(ToolResponse::success(Value::Number(
            serde_json::Number::from_f64(result).unwrap(),
        )))
    }
}
