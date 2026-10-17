//! Example tool implementations.

use super::tool::{Tool, ToolError, ToolRequest, ToolResponse};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

/// Tool that echoes the input back.
pub struct EchoTool;

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

    fn handle(
        &self,
        request: ToolRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResponse, ToolError>> + Send>> {
        Box::pin(async move { Ok(ToolResponse::success(request.input)) })
    }
}

/// Calculator tool supporting basic arithmetic.
pub struct CalcTool;

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
                let safe_op = if op.len() > 32 || op.chars().any(|c| c.is_control()) {
                    "invalid".to_string()
                } else {
                    op.to_string()
                };
                return Err(ToolError::InvalidInput(format!(
                    "Unsupported operation: {safe_op}"
                )));
            }
        }

        if !obj
            .get("a")
            .and_then(|v| v.as_f64())
            .is_some_and(|n| n.is_finite())
        {
            return Err(ToolError::InvalidInput(
                "Missing or invalid 'a' field".to_string(),
            ));
        }

        if !obj
            .get("b")
            .and_then(|v| v.as_f64())
            .is_some_and(|n| n.is_finite())
        {
            return Err(ToolError::InvalidInput(
                "Missing or invalid 'b' field".to_string(),
            ));
        }

        Ok(())
    }

    fn handle(
        &self,
        request: ToolRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResponse, ToolError>> + Send>> {
        Box::pin(async move {
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
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

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

    #[tokio::test]
    async fn test_calc_validation_errors() {
        let tool = CalcTool;

        // Not an object
        let res = tool.validate(&serde_json::json!(123));
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("Expected object"));

        // Missing op
        let res = tool.validate(&serde_json::json!({"a": 1, "b": 2}));
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("Missing or invalid 'op'")
        );

        // Invalid op type
        let res = tool.validate(&serde_json::json!({"op": 1, "a": 1, "b": 2}));
        assert!(res.is_err());

        // Unsupported operation
        let res = tool.validate(&serde_json::json!({"op": "pow", "a": 1, "b": 2}));
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("Unsupported operation")
        );

        // Missing 'a'
        let res = tool.validate(&serde_json::json!({"op": "add", "b": 2}));
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("Missing or invalid 'a'")
        );

        // Invalid 'a' type
        let res = tool.validate(&serde_json::json!({"op": "add", "a": "1", "b": 2}));
        assert!(res.is_err());

        // Missing 'b'
        let res = tool.validate(&serde_json::json!({"op": "add", "a": 1}));
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("Missing or invalid 'b'")
        );
    }

    #[tokio::test]
    async fn test_calc_division_by_zero() {
        let tool = CalcTool;
        let request = ToolRequest::new(serde_json::json!({
            "op": "div",
            "a": 10.0,
            "b": 0.0
        }));
        let res = tool.handle(request).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("Division by zero"));
    }

    #[tokio::test]
    async fn test_calc_validate_unsupported_op_sanitization() {
        let tool = CalcTool;

        // Untrusted op with control characters
        let res = tool.validate(&serde_json::json!({"op": "add\n[INJECTED]", "a": 1, "b": 2}));
        assert!(res.is_err());
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains("Unsupported operation: invalid"));
        assert!(!err_msg.contains('\n'));

        // Oversized op string (> 32 chars)
        let long_op = "a".repeat(33);
        let res = tool.validate(&serde_json::json!({"op": long_op, "a": 1, "b": 2}));
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("Unsupported operation: invalid")
        );

        // Valid short unsupported op name
        let res = tool.validate(&serde_json::json!({"op": "power", "a": 1, "b": 2}));
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("Unsupported operation: power")
        );
    }

    #[tokio::test]
    async fn test_calc_validate_numeric_fields() {
        let tool = CalcTool;

        // Non-numeric string in 'a'
        let res = tool.validate(&serde_json::json!({"op": "add", "a": "10", "b": 1}));
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("Missing or invalid 'a'")
        );

        // Non-numeric boolean in 'b'
        let res = tool.validate(&serde_json::json!({"op": "add", "a": 1, "b": true}));
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("Missing or invalid 'b'")
        );

        // Valid finite numbers pass validation
        let res = tool.validate(&serde_json::json!({"op": "add", "a": 1.5, "b": -2.5}));
        assert!(res.is_ok());
    }
}
