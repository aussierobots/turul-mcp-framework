//! Comprehensive SessionContext Macro Integration Tests
//!
//! This module tests SessionContext functionality with both derive and function macros:
//! - SessionContext parameter passing in derive macros
//! - SessionContext parameter passing in function macros
//! - Session state management through macros
//! - Progress notifications from macro-generated tools
//! - Error handling when SessionContext is missing
//!
//! NOTE: These tests now use proper SessionContext creation via test helpers.

use serde_json::{json, Value};
use tokio;

use turul_mcp_derive::{McpTool, mcp_tool};
use turul_mcp_server::{McpResult, SessionContext, McpTool};

mod test_helpers;
use test_helpers::{TestSessionBuilder, create_test_session, create_test_session_pair, assert_session_state, assert_notification_sent};
use turul_mcp_protocol::McpError;

/// Test derive macro with SessionContext support
#[derive(McpTool, Default)]
#[tool(name = "test_derive_with_session", description = "Test derive macro with SessionContext")]
struct TestDeriveWithSession {
    #[param(description = "Test input")]
    input: String,
}

impl TestDeriveWithSession {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| McpError::SessionError("Session required for testing".to_string()))?;
        
        // Test state management
        let count: i32 = session.get_typed_state("call_count").unwrap_or(0);
        let new_count = count + 1;
        session.set_typed_state("call_count", &new_count).unwrap();
        
        // Test progress notifications
        session.notify_progress("processing", new_count as u64);
        
        Ok(json!({
            "input": self.input,
            "call_count": new_count,
            "session_id": session.session_id,
            "message": format!("Processed '{}' (call #{})", self.input, new_count)
        }))
    }
}

/// Test derive macro without SessionContext (backward compatibility)
#[derive(McpTool, Default)]
#[tool(name = "test_derive_no_session", description = "Test derive macro without SessionContext")]
struct TestDeriveNoSession {
    #[param(description = "Test input")]
    input: String,
}

impl TestDeriveNoSession {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<Value> {
        // This should work even without using the session
        Ok(json!({
            "input": self.input,
            "message": "No session needed"
        }))
    }
}

// Function macro with SessionContext
#[mcp_tool(name = "test_function_with_session", description = "Test function macro with SessionContext")]
async fn test_function_with_session(
    #[param(description = "Test input")] input: String,
    session: Option<SessionContext>
) -> McpResult<String> {
    let session = session.ok_or_else(|| McpError::SessionError("Session required for function test".to_string()))?;
    
    // Test state management in function macro
    let prefix: String = session.get_typed_state("prefix").unwrap_or("default".to_string());
    session.set_typed_state("last_input", &input).unwrap();
    
    // Test progress notification
    session.notify_progress("function_processing", 1);
    
    Ok(format!("{}: {}", prefix, input))
}

// Function macro without SessionContext (backward compatibility)
#[mcp_tool(name = "test_function_no_session", description = "Test function macro without SessionContext")]
async fn test_function_no_session(
    #[param(description = "Test input")] input: String,
) -> McpResult<String> {
    Ok(format!("Simple: {}", input))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    /// Create a test SessionContext using the test helpers
    async fn create_test_session() -> SessionContext {
        test_helpers::create_test_session().await
    }
    
    #[tokio::test]
    async fn test_derive_macro_with_session_context() {
        let session = create_test_session().await;
        let tool = TestDeriveWithSession::default();
        
        // First call
        let args = json!({"input": "hello"});
        let result = tool.call(args.clone(), Some(session.clone())).await.unwrap();
        
        // Verify the result structure
        let content = match &result.content[0] {
            turul_mcp_protocol::tools::ToolResult::Text { text } => text,
            _ => panic!("Expected text content")
        };
        let response: Value = serde_json::from_str(content).unwrap();
        
        assert_eq!(response["value"]["input"], "hello");
        assert_eq!(response["value"]["call_count"], 1);
        assert!(!response["value"]["session_id"].as_str().unwrap().is_empty());
        
        // Second call should increment counter
        let args2 = json!({"input": "world"});
        let result2 = tool.call(args2, Some(session.clone())).await.unwrap();
        let content2 = match &result2.content[0] {
            turul_mcp_protocol::tools::ToolResult::Text { text } => text,
            _ => panic!("Expected text content")
        };
        let response2: Value = serde_json::from_str(content2).unwrap();
        
        assert_eq!(response2["value"]["call_count"], 2);
        assert_eq!(response2["value"]["input"], "world");
    }
    
    #[tokio::test]
    async fn test_derive_macro_without_session_context() {
        let tool = TestDeriveNoSession::default();
        
        // Should work with None session
        let args = json!({"input": "test"});
        let result = tool.call(args, None).await.unwrap();
        
        let content = match &result.content[0] {
            turul_mcp_protocol::tools::ToolResult::Text { text } => text,
            _ => panic!("Expected text content")
        };
        let response: Value = serde_json::from_str(content).unwrap();
        
        assert_eq!(response["value"]["input"], "test");
        assert_eq!(response["value"]["message"], "No session needed");
    }
    
    #[tokio::test]
    async fn test_derive_macro_session_required_error() {
        let tool = TestDeriveWithSession::default();
        
        // Should fail without session
        let args = json!({"input": "fail"});
        let result = tool.call(args, None).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            McpError::SessionError(msg) => {
                assert!(msg.contains("Session required"));
            }
            _ => panic!("Expected SessionError")
        }
    }
    
    #[tokio::test]
    async fn test_function_macro_with_session_context() {
        let session = create_test_session().await;
        let tool = test_function_with_session();
        
        // Set up session state
        session.set_typed_state("prefix", &"TEST".to_string()).unwrap();
        
        let args = json!({"input": "function"});
        let result = tool.call(args, Some(session.clone())).await.unwrap();
        
        let content = match &result.content[0] {
            turul_mcp_protocol::tools::ToolResult::Text { text } => text,
            _ => panic!("Expected text content")
        };
        let response: Value = serde_json::from_str(content).unwrap();
        
        // Function macro wraps result in output field (default "result")
        assert_eq!(response["result"], "TEST: function");
        
        // Verify state was set
        let last_input: String = session.get_typed_state("last_input").unwrap();
        assert_eq!(last_input, "function");
    }
    
    #[tokio::test]
    async fn test_function_macro_without_session_context() {
        let tool = test_function_no_session();
        
        // Should work with None session
        let args = json!({"input": "simple"});
        let result = tool.call(args, None).await.unwrap();
        
        let content = match &result.content[0] {
            turul_mcp_protocol::tools::ToolResult::Text { text } => text,
            _ => panic!("Expected text content")
        };
        let response: Value = serde_json::from_str(content).unwrap();
        
        assert_eq!(response["result"], "Simple: simple");
    }
    
    #[tokio::test]
    async fn test_function_macro_session_required_error() {
        let tool = test_function_with_session();
        
        // Should fail without session
        let args = json!({"input": "fail"});
        let result = tool.call(args, None).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            McpError::ToolExecutionError(msg) => {
                assert!(msg.contains("Session required"));
            }
            other => {
                println!("Got error: {:?}", other);
                panic!("Expected ToolExecutionError containing session error, got: {:?}", other)
            }
        }
    }
    
    #[tokio::test]
    async fn test_session_state_persistence_across_tools() {
        let session = create_test_session().await;
        
        // Use derive macro tool to set state
        let derive_tool = TestDeriveWithSession::default();
        let args1 = json!({"input": "first"});
        derive_tool.call(args1, Some(session.clone())).await.unwrap();
        
        // Use function macro tool to read state
        session.set_typed_state("prefix", &"SHARED".to_string()).unwrap();
        let function_tool = test_function_with_session();
        let args2 = json!({"input": "second"});
        let result = function_tool.call(args2, Some(session.clone())).await.unwrap();
        println!("Function call result: {:?}", result); // Debug statement for result usage
        
        // Both tools should share session state
        let last_input: String = session.get_typed_state("last_input").unwrap();
        assert_eq!(last_input, "second");
        
        let call_count: i32 = session.get_typed_state("call_count").unwrap();
        assert_eq!(call_count, 1); // Set by derive tool
    }
    
    #[tokio::test]
    async fn test_macro_progress_notifications() {
        let session = create_test_session().await;
        
        // Both macro types should be able to send progress notifications
        let derive_tool = TestDeriveWithSession::default();
        let function_tool = test_function_with_session();
        
        // Test derive macro notifications
        let args1 = json!({"input": "progress1"});
        derive_tool.call(args1, Some(session.clone())).await.unwrap();
        
        // Test function macro notifications
        let args2 = json!({"input": "progress2"});
        function_tool.call(args2, Some(session.clone())).await.unwrap();
        
        // This test mainly verifies no panics occur during notification sending
        // In a real implementation, you'd verify the notifications were actually sent
    }
}