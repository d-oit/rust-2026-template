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

    fn validate(&self, input: &Value) -> Result<(), ToolError> {
        if input.is_null() {
            return Err(ToolError::InvalidInput("Input cannot be null".to_string()));
        }
        Ok(())
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
        let obj = input
            .as_object()
            .ok_or_else(|| ToolError::InvalidInput("Expected object".to_string()))?;

        let op = obj
            .get("op")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("Missing or invalid 'op' field".to_string()))?;

        match op {
            "add" | "sub" | "mul" | "div" => {}
            _ => {
                return Err(ToolError::InvalidInput(format!(
                    "Unsupported operation: {op}"
                )));
            }
        }

        if !obj.get("a").is_some_and(|v| v.is_number()) {
            return Err(ToolError::InvalidInput(
                "Missing or invalid 'a' field".to_string(),
            ));
        }

        if !obj.get("b").is_some_and(|v| v.is_number()) {
            return Err(ToolError::InvalidInput(
                "Missing or invalid 'b' field".to_string(),
            ));
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

        let result_num = serde_json::Number::from_f64(result)
            .ok_or_else(|| ToolError::Execution("Result is not a finite number".to_string()))?;

        Ok(ToolResponse::success(Value::Number(result_num)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_calc_no_panic_on_overflow() {
        let tool = CalcTool;
        let request = ToolRequest::new(serde_json::json!({
            "op": "mul",
            "a": 1e308,
            "b": 1e308
        }));
        let result = tool.handle(request).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not a finite number")
        );
    }
}
