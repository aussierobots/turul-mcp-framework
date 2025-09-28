//! Basic SessionContext functionality test
//!
//! This is a minimal test to verify that SessionContext is working
//! with the updated macro implementations.

use serde_json::json;
use turul_mcp_derive::{McpTool, mcp_tool};
use turul_mcp_server::{McpResult, SessionContext};

/// Simple test to verify derive macro SessionContext passing
#[derive(McpTool, Default)]
#[tool(
    name = "session_test",
    description = "Test SessionContext in derive macro"
)]
struct SessionTestTool {
    #[param(description = "Test input")]
    input: String,
}

impl SessionTestTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<serde_json::Value> {
        Ok(json!({
            "input": self.input,
            "has_session": session.is_some(),
            "session_id": session.map(|s| s.session_id).unwrap_or_else(|| "no-session".to_string())
        }))
    }
}

/// Simple test to verify function macro SessionContext passing
#[mcp_tool(
    name = "session_function_test",
    description = "Test SessionContext in function macro"
)]
async fn session_function_test(
    #[param(description = "Test input")] input: String,
    session: Option<SessionContext>,
) -> McpResult<String> {
    Ok(format!(
        "input: {}, has_session: {}",
        input,
        session.is_some()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use turul_mcp_server::McpTool;

    #[tokio::test]
    async fn test_derive_macro_session_support() {
        let tool = SessionTestTool::default();
        let args = json!({"input": "test"});

        // Test with None session (should work)
        let result = tool.call(args, None).await.unwrap();
        assert!(!result.content.is_empty());

        // Parse the response to check session handling
        let content = match &result.content[0] {
            turul_mcp_protocol::tools::ToolResult::Text { text, .. } => text,
            _ => panic!("Expected text content"),
        };
        let response: Value = serde_json::from_str(content).unwrap();

        assert_eq!(response["value"]["input"], "test");
        assert_eq!(response["value"]["has_session"], false);
        assert_eq!(response["value"]["session_id"], "no-session");
    }

    #[tokio::test]
    async fn test_function_macro_session_support() {
        let tool = session_function_test();
        let args = json!({"input": "function_test"});

        // Test with None session (should work)
        let result = tool.call(args, None).await.unwrap();
        assert!(!result.content.is_empty());

        // Parse the response (function macro wraps result in "result" field)
        let content = match &result.content[0] {
            turul_mcp_protocol::tools::ToolResult::Text { text, .. } => text,
            _ => panic!("Expected text content"),
        };
        let response: Value = serde_json::from_str(content).unwrap();

        assert!(
            response["result"]
                .as_str()
                .unwrap()
                .contains("function_test")
        );
        assert!(
            response["result"]
                .as_str()
                .unwrap()
                .contains("has_session: false")
        );
    }
}
