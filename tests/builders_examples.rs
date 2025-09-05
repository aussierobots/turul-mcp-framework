//! Tests for code examples from turul-mcp-builders README.md
//!
//! These tests verify that all builder pattern examples from the turul-mcp-builders README
//! compile correctly and integrate properly with the server framework.

use turul_mcp_server::{McpServer, ToolBuilder};
use turul_mcp_builders::ResourceBuilder;
use serde_json::json;

/// Test basic ToolBuilder server integration example from turul-mcp-builders README
#[test]
fn test_server_integration_toolbuilder() {
    let calculator = ToolBuilder::new("calculator")
        .description("Add two numbers")
        .number_param("a", "First number")
        .number_param("b", "Second number")
        .number_output()
        .execute(|args| async move {
            let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
            Ok(json!({"result": a + b}))
        })
        .build()
        .expect("Tool should build successfully");

    // Use in server
    let _server = McpServer::builder()
        .name("builders-test-server")
        .version("1.0.0")
        .tool(calculator)
        .bind_address("127.0.0.1:8080".parse().unwrap())
        .build()
        .expect("Server should build successfully");
}

/// Test dynamic tool construction example from turul-mcp-builders README
/// Note: This uses turul_mcp_builders::ToolBuilder directly (not the server wrapper)
#[test]
fn test_dynamic_tool_construction() {
    use turul_mcp_builders::ToolBuilder as DirectToolBuilder;

    // This would be used when building tools dynamically without server integration
    let _tool = DirectToolBuilder::new("data_processor")
        .description("Process data with custom logic")
        .string_param("input", "Input data to process")
        .string_param("format", "Output format (json, csv, xml)")
        .boolean_param("validate", "Validate input data")
        .execute(|args| async move {
            let input = args.get("input").and_then(|v| v.as_str()).unwrap_or("");
            let format = args.get("format").and_then(|v| v.as_str()).unwrap_or("json");
            let validate = args.get("validate").and_then(|v| v.as_bool()).unwrap_or(false);
            
            // Mock processing logic for test
            if validate && input.is_empty() {
                return Err("Input cannot be empty".into());
            }
            
            let result = format!("Processed {} as {}", input, format);
            Ok(json!({"processed": result, "format": format}))
        })
        .build()
        .expect("Tool should build successfully");
}

/// Test ResourceBuilder example from turul-mcp-builders README
#[test]  
fn test_resource_builder() {
    let _config_resource = ResourceBuilder::new("file:///app/config.json")
        .name("app_config")
        .description("Application configuration")
        .json_content(json!({
            "version": "1.0.0",
            "features": ["logging", "metrics", "auth"]
        }))
        .build()
        .expect("Resource should build successfully");
}

/// Test ToolBuilder with various parameter types
#[test]
fn test_toolbuilder_parameter_types() {
    let _complex_tool = ToolBuilder::new("complex_tool")
        .description("Tool with various parameter types")
        .string_param("text", "String parameter")
        .number_param("count", "Number parameter")
        .boolean_param("enabled", "Boolean parameter")
        .string_param("optional", "Optional parameter")
        .execute(|args| async move {
            let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let count = args.get("count").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let enabled = args.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
            let optional = args.get("optional").and_then(|v| v.as_str());
            
            Ok(json!({
                "result": {
                    "text": text,
                    "count": count,
                    "enabled": enabled,
                    "optional": optional
                }
            }))
        })
        .build()
        .expect("Complex tool should build successfully");
}

/// Test error handling in ToolBuilder
#[test]
fn test_toolbuilder_error_handling() {
    let _error_tool = ToolBuilder::new("error_tool")
        .description("Tool that demonstrates error handling")
        .string_param("input", "Input that might cause errors")
        .execute(|args| async move {
            let input = args.get("input").and_then(|v| v.as_str()).unwrap_or("");
            
            if input == "error" {
                return Err("Intentional error for testing".into());
            }
            
            if input.is_empty() {
                return Err("Input cannot be empty".into());
            }
            
            Ok(json!({"processed": input}))
        })
        .build()
        .expect("Error handling tool should build successfully");
}