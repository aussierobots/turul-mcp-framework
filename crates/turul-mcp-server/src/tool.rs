//! MCP Tool Trait
//!
//! This module defines the high-level trait for implementing MCP tools.

use async_trait::async_trait;
use serde_json::Value;
use turul_mcp_builders::prelude::*;
use turul_mcp_protocol::{CallToolResult, McpResult};

use crate::session::SessionContext;

/// High-level trait for implementing MCP tools
///
/// McpTool extends ToolDefinition with execution capabilities.
/// All metadata is provided by the ToolDefinition trait, ensuring
/// consistency between concrete Tool structs and dynamic implementations.
#[async_trait]
pub trait McpTool: ToolDefinition {
    /// Execute the tool with full session support
    ///
    /// This is the primary execution method that tools should implement.
    /// Returns a complete CallToolResponse with both content and structured data.
    async fn call(&self, args: Value, session: Option<SessionContext>)
    -> McpResult<CallToolResult>;
}

/// Converts an McpTool trait object to a protocol Tool descriptor
///
/// This is now a thin wrapper around the ToolDefinition::to_tool() method
/// for backward compatibility. New code should use tool.to_tool() directly.
pub fn tool_to_descriptor(tool: &dyn McpTool) -> turul_mcp_protocol::Tool {
    tool.to_tool()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use turul_mcp_protocol::schema::JsonSchema;
    use turul_mcp_protocol::tools::{CallToolResult, ToolAnnotations, ToolResult, ToolSchema};
    // Framework traits already imported via prelude at module level

    struct TestTool {
        input_schema: ToolSchema,
    }

    impl TestTool {
        fn new() -> Self {
            let input_schema = ToolSchema::object()
                .with_properties(HashMap::from([(
                    "message".to_string(),
                    JsonSchema::string(),
                )]))
                .with_required(vec!["message".to_string()]);
            Self { input_schema }
        }
    }

    // Implement all the fine-grained traits
    impl HasBaseMetadata for TestTool {
        fn name(&self) -> &str {
            "test"
        }
        fn title(&self) -> Option<&str> {
            None
        }
    }

    impl HasDescription for TestTool {
        fn description(&self) -> Option<&str> {
            Some("A test tool")
        }
    }

    impl HasInputSchema for TestTool {
        fn input_schema(&self) -> &ToolSchema {
            &self.input_schema
        }
    }

    impl HasOutputSchema for TestTool {
        fn output_schema(&self) -> Option<&ToolSchema> {
            None
        }
    }

    impl HasAnnotations for TestTool {
        fn annotations(&self) -> Option<&ToolAnnotations> {
            None
        }
    }

    impl HasToolMeta for TestTool {
        fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
            None
        }
    }

    // ToolDefinition is automatically implemented via blanket impl!

    #[async_trait]
    impl McpTool for TestTool {
        async fn call(
            &self,
            args: Value,
            _session: Option<SessionContext>,
        ) -> McpResult<CallToolResult> {
            let message = args
                .get("message")
                .and_then(|v| v.as_str())
                .ok_or_else(|| turul_mcp_protocol::McpError::missing_param("message"))?;

            let result = format!("Test: {}", message);
            Ok(CallToolResult::success(vec![ToolResult::text(result)]))
        }
    }

    #[test]
    fn test_tool_trait() {
        let tool = TestTool::new();
        assert_eq!(tool.name(), "test");
        assert_eq!(tool.description(), Some("A test tool"));
        assert!(tool.annotations().is_none());
    }

    #[test]
    fn test_tool_conversion() {
        let tool = TestTool::new();
        let mcp_tool = tool_to_descriptor(&tool);

        assert_eq!(mcp_tool.name, "test");
        assert_eq!(mcp_tool.description, Some("A test tool".to_string()));
        // ToolSchema doesn't have schema_type field anymore, check structure instead
        assert!(mcp_tool.input_schema.properties.is_some());
    }

    #[tokio::test]
    async fn test_tool_call() {
        let tool = TestTool::new();
        let args = serde_json::json!({"message": "hello"});

        let result = tool.call(args, None).await.unwrap();
        assert!(!result.content.is_empty());

        let ToolResult::Text { text, .. } = &result.content[0] else {
            panic!("Expected text result, got: {:?}", result.content[0]);
        };
        assert_eq!(text, "Test: hello");
    }

    #[tokio::test]
    async fn test_tool_call_error() {
        let tool = TestTool::new();
        let args = serde_json::json!({"wrong": "parameter"});

        let result = tool.call(args, None).await;
        assert!(result.is_err());

        let error = result.unwrap_err();
        let turul_mcp_protocol::McpError::MissingParameter(param) = error else {
            panic!("Expected MissingParameter error, got: {:?}", error);
        };
        assert_eq!(param, "message");
    }
}
