//! # Proper Framework Integration Tests
//!
//! These tests use the framework's actual APIs and typed structures,
//! not raw JSON manipulation.

use serde_json::{Value, json};
use std::collections::HashMap;
use turul_mcp_derive::{McpTool, mcp_tool};
use turul_mcp_protocol::tools::{CallToolParams, CallToolRequest, ToolResult};
use turul_mcp_server::{McpResult, McpServerBuilder, McpTool, SessionContext, ToolBuilder};

/// Test tool using derive macro - proper framework usage
#[derive(McpTool, Default)]
#[tool(name = "calculator", description = "Add two numbers")]
struct CalculatorTool {
    #[param(description = "First number")]
    a: f64,
    #[param(description = "Second number")]
    b: f64,
}

impl CalculatorTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<f64> {
        Ok(self.a + self.b)
    }
}

/// Test tool using function macro - proper framework usage
#[mcp_tool(name = "multiply", description = "Multiply two numbers")]
async fn multiply_tool(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
    _session: Option<SessionContext>,
) -> McpResult<f64> {
    Ok(a * b)
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_framework_tool_creation_and_execution() {
        // Test 1: Derive macro tool
        let calc_tool = CalculatorTool { a: 5.0, b: 3.0 };

        // Use framework's tool trait, not raw JSON
        let result = calc_tool
            .call(json!({"a": 5.0, "b": 3.0}), None)
            .await
            .unwrap();

        // Verify using framework types
        assert_eq!(result.content.len(), 1);
        match &result.content[0] {
            ToolResult::Text { text, .. } => {
                let parsed: Value = serde_json::from_str(text).unwrap();
                // Derive macro uses "output" as the default field name
                assert_eq!(parsed["output"], 8.0);
            }
            _ => panic!("Expected text result"),
        }
    }

    #[tokio::test]
    async fn test_framework_function_macro_tool() {
        // Test 2: Function macro tool
        let multiply = multiply_tool();

        let result = multiply
            .call(json!({"a": 4.0, "b": 6.0}), None)
            .await
            .unwrap();

        assert_eq!(result.content.len(), 1);
        match &result.content[0] {
            ToolResult::Text { text, .. } => {
                let parsed: Value = serde_json::from_str(text).unwrap();
                assert_eq!(parsed["result"], 24.0);
            }
            _ => panic!("Expected text result"),
        }
    }

    #[tokio::test]
    async fn test_framework_builder_pattern_tool() {
        // Test 3: Builder pattern - using framework builders
        let tool = ToolBuilder::new("division")
            .description("Divide two numbers")
            .number_param("dividend", "Number to divide")
            .number_param("divisor", "Number to divide by")
            .execute(|args| async move {
                let a = args.get("dividend").and_then(|v| v.as_f64()).unwrap();
                let b = args.get("divisor").and_then(|v| v.as_f64()).unwrap();
                if b == 0.0 {
                    return Err("Division by zero".into());
                }
                Ok(json!({"result": a / b}))
            })
            .build()
            .unwrap();

        let result = tool
            .call(json!({"dividend": 15.0, "divisor": 3.0}), None)
            .await
            .unwrap();

        assert_eq!(result.content.len(), 1);
    }

    #[tokio::test]
    async fn test_framework_server_integration() {
        // Test 4: Full server integration using framework APIs
        let _server = McpServerBuilder::new()
            .name("test-server")
            .version("1.0.0")
            .tool(CalculatorTool::default())
            .tool(multiply_tool())
            .build()
            .unwrap();

        // Note: Server integration tests would require HTTP server setup
        // For now, just verify the server builds successfully
        // Server built successfully - continuing with test
    }

    #[tokio::test]
    async fn test_framework_protocol_request_handling() {
        // Test 5: Proper protocol types construction
        let mut args_map = HashMap::new();
        args_map.insert("a".to_string(), json!(10.0));
        args_map.insert("b".to_string(), json!(5.0));

        let _call_request = CallToolRequest {
            method: "tools/call".to_string(),
            params: CallToolParams {
                name: "calculator".to_string(),
                arguments: Some(args_map),
                task: None,
                meta: None,
            },
        };

        // Note: Protocol request handling would require full server dispatch
        // This test verifies proper type construction
        // Request constructed successfully - continuing with test
    }

    #[tokio::test]
    async fn test_framework_error_handling() {
        // Test 6: Proper error handling using framework types
        let tool = ToolBuilder::new("error_test")
            .description("Tool that errors")
            .execute(|_| async move { Err("Intentional test error".into()) })
            .build()
            .unwrap();

        let result = tool.call(json!({}), None).await;

        // Use framework error types
        assert!(result.is_err());
        match result.unwrap_err() {
            turul_mcp_protocol::McpError::ToolExecutionError(msg) => {
                assert!(msg.contains("Intentional test error"));
            }
            _ => panic!("Expected ToolExecutionError"),
        }
    }

    #[tokio::test]
    async fn test_framework_session_integration() {
        // Test 7: Basic SessionContext usage (without test helpers for now)
        let tool = CalculatorTool { a: 7.0, b: 2.0 };

        // Test with None session (should still work for basic tools)
        let result = tool.call(json!({"a": 7.0, "b": 2.0}), None).await.unwrap();

        // Verify result structure
        assert_eq!(result.content.len(), 1);
        match &result.content[0] {
            ToolResult::Text { text, .. } => {
                let parsed: Value = serde_json::from_str(text).unwrap();
                // Derive macro uses "output" as the default field name
                assert_eq!(parsed["output"], 9.0);
            }
            _ => panic!("Expected text result"),
        }
    }
}
