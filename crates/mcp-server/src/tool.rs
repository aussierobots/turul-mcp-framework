//! MCP Tool Trait
//!
//! This module defines the high-level trait for implementing MCP tools.

use async_trait::async_trait;
use mcp_protocol::{ToolSchema, ToolResult, McpResult};
use serde_json::Value;

use crate::session::SessionContext;

#[cfg(test)]
use mcp_protocol::McpError;

/// High-level trait for implementing MCP tools
#[async_trait]
pub trait McpTool: Send + Sync {
    /// The name of the tool - used as the identifier
    fn name(&self) -> &str;

    /// Human-readable description of what the tool does
    fn description(&self) -> &str;

    /// JSON Schema describing the tool's input parameters
    fn input_schema(&self) -> ToolSchema;

    /// Execute the tool with the given arguments and optional session context
    /// 
    /// The session context is automatically provided by the framework when available.
    /// Tools can use it for state persistence across multiple calls.
    /// 
    /// Returns a list of content items or a structured MCP error
    async fn call(
        &self, 
        args: Value,
        session: Option<SessionContext>,
    ) -> McpResult<Vec<ToolResult>>;

    /// Optional: Get tool annotations for client hints
    fn annotations(&self) -> Option<Value> {
        None
    }
}

/// Convert an McpTool trait object to a Tool descriptor
pub fn tool_to_descriptor(tool: &dyn McpTool) -> mcp_protocol::Tool {
    let mut mcp_tool = mcp_protocol::Tool::new(tool.name(), tool.input_schema())
        .with_description(tool.description());

    if let Some(annotations) = tool.annotations() {
        mcp_tool = mcp_tool.with_annotations(annotations);
    }

    mcp_tool
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use mcp_protocol::schema::JsonSchema;

    struct TestTool;

    #[async_trait]
    impl McpTool for TestTool {
        fn name(&self) -> &str {
            "test"
        }

        fn description(&self) -> &str {
            "A test tool"
        }

        fn input_schema(&self) -> ToolSchema {
            ToolSchema::object()
                .with_properties(HashMap::from([
                    ("message".to_string(), JsonSchema::string()),
                ]))
                .with_required(vec!["message".to_string()])
        }

        async fn call(&self, args: Value, _session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
            let message = args.get("message")
                .and_then(|v| v.as_str())
                .ok_or_else(|| McpError::missing_param("message"))?;

            Ok(vec![ToolResult::text(format!("Test: {}", message))])
        }
    }

    #[test]
    fn test_tool_trait() {
        let tool = TestTool;
        assert_eq!(tool.name(), "test");
        assert_eq!(tool.description(), "A test tool");
        assert!(tool.annotations().is_none());
    }

    #[test]
    fn test_tool_conversion() {
        let tool = TestTool;
        let mcp_tool = tool_to_descriptor(&tool);
        
        assert_eq!(mcp_tool.name, "test");
        assert_eq!(mcp_tool.description, Some("A test tool".to_string()));
        assert_eq!(mcp_tool.input_schema.schema_type, "object");
    }

    #[tokio::test]
    async fn test_tool_call() {
        let tool = TestTool;
        let args = serde_json::json!({"message": "hello"});
        
        let result = tool.call(args, None).await.unwrap();
        assert_eq!(result.len(), 1);
        
        if let ToolResult::Text { text } = &result[0] {
            assert_eq!(text, "Test: hello");
        } else {
            panic!("Expected text result");
        }
    }

    #[tokio::test]
    async fn test_tool_call_error() {
        let tool = TestTool;
        let args = serde_json::json!({"wrong": "parameter"});
        
        let result = tool.call(args, None).await;
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        if let McpError::MissingParameter(param) = error {
            assert_eq!(param, "message");
        } else {
            panic!("Expected MissingParameter error, got: {:?}", error);
        }
    }
}