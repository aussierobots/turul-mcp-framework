//! Test for custom output field name feature

use mcp_derive::mcp_tool;
use mcp_server::{McpTool, McpResult};
use serde_json::json;

#[mcp_tool(name = "test_custom_field", description = "Test custom output field", output_field = "sum")]
async fn test_custom_field_tool(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<f64> {
    Ok(a + b)
}

#[tokio::test]
async fn test_custom_output_field_name() {
    let tool = test_custom_field_tool();
    let args = json!({"a": 5.0, "b": 3.0});
    
    let result = tool.call(args, None).await.unwrap();
    
    // Verify structured content uses "sum" instead of "result"
    assert!(result.structured_content.is_some());
    if let Some(structured) = result.structured_content {
        // Should have "sum" field, not "result" field
        assert!(structured.get("sum").is_some());
        assert!(structured.get("result").is_none());
        
        let sum_value = structured.get("sum").unwrap().as_f64().unwrap();
        assert_eq!(sum_value, 8.0);
    }
    
    // Verify basic properties
    assert!(!result.content.is_empty());
    assert_eq!(result.is_error, Some(false));
}

#[mcp_tool(name = "test_default_field", description = "Test default output field")]
async fn test_default_field_tool(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<f64> {
    Ok(a + b)
}

#[tokio::test]
async fn test_default_output_field_name() {
    let tool = test_default_field_tool();
    let args = json!({"a": 7.0, "b": 2.0});
    
    let result = tool.call(args, None).await.unwrap();
    
    // Verify structured content uses default "result" field
    assert!(result.structured_content.is_some());
    if let Some(structured) = result.structured_content {
        // Should have "result" field by default
        assert!(structured.get("result").is_some());
        assert!(structured.get("sum").is_none());
        
        let result_value = structured.get("result").unwrap().as_f64().unwrap();
        assert_eq!(result_value, 9.0);
    }
}