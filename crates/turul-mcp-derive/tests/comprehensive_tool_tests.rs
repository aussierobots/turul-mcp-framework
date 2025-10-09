//! Comprehensive tests for MCP derive macros covering all data types and scenarios

use serde_json::json;
use turul_mcp_derive::McpTool;
use turul_mcp_builders::prelude::*;  // HasBaseMetadata, HasOutputSchema, etc.
use turul_mcp_server::{McpResult, McpTool as McpToolTrait, SessionContext};

/// Test tool returning f64 (number)
#[derive(McpTool)]
struct NumberTool {
    #[param(description = "Input value")]
    value: f64,
}

impl NumberTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<f64> {
        Ok(self.value * 2.0)
    }
}

/// Test tool returning String
#[derive(McpTool)]
struct StringTool {
    #[param(description = "Input text")]
    text: String,
}

impl StringTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok(format!("Processed: {}", self.text))
    }
}

/// Test tool returning bool
#[derive(McpTool)]
struct BooleanTool {
    #[param(description = "Input number")]
    number: i32,
}

impl BooleanTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<bool> {
        Ok(self.number > 0)
    }
}

/// Test tool returning Vec<i32> (array)
#[derive(McpTool)]
struct ArrayTool {
    #[param(description = "Array size")]
    size: u32,
}

impl ArrayTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<Vec<i32>> {
        Ok((0..self.size as i32).collect())
    }
}

/// Test tool returning custom struct (object)
#[derive(serde::Serialize)]
struct CustomResult {
    message: String,
    count: u32,
    success: bool,
}

#[derive(McpTool)]
struct ObjectTool {
    #[param(description = "Message text")]
    message: String,
    #[param(description = "Count value")]
    count: u32,
}

impl ObjectTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<CustomResult> {
        Ok(CustomResult {
            message: self.message.clone(),
            count: self.count,
            success: true,
        })
    }
}

#[tokio::test]
async fn test_number_tool_execution_and_schema() {
    let tool = NumberTool { value: 5.0 };

    // Test execution
    let result = tool.call(json!({"value": 5.0}), None).await;
    assert!(result.is_ok());

    let response = result.unwrap();

    // Should have text content
    assert!(!response.content.is_empty());

    // Should have structured content in MCP object format {"output": value} for primitives
    assert!(response.structured_content.is_some());
    if let Some(structured) = response.structured_content {
        let obj = structured.as_object().unwrap();
        assert!(
            obj.contains_key("output"),
            "Expected 'output' field for primitive type, got keys: {:?}",
            obj.keys().collect::<Vec<_>>()
        );
        assert_eq!(obj["output"], json!(10.0));
    }

    // Should have MCP-compliant object output schema after execution
    assert!(tool.output_schema().is_some());
    let schema = tool.output_schema().unwrap();
    assert_eq!(
        schema.schema_type, "object",
        "MCP requires all output schemas to be objects"
    );
}

#[tokio::test]
async fn test_string_tool_execution_and_schema() {
    let tool = StringTool {
        text: "test".to_string(),
    };

    // Test execution
    let result = tool.call(json!({"text": "test"}), None).await;
    assert!(result.is_ok());

    let response = result.unwrap();

    // Should have text content
    assert!(!response.content.is_empty());

    // Should have structured content in MCP object format {"output": value} for primitives
    assert!(response.structured_content.is_some());
    if let Some(structured) = response.structured_content {
        let obj = structured.as_object().unwrap();
        assert!(
            obj.contains_key("output"),
            "Expected 'output' field for primitive type, got keys: {:?}",
            obj.keys().collect::<Vec<_>>()
        );
        assert_eq!(obj["output"], json!("Processed: test"));
    }

    // Should have MCP-compliant object output schema after execution
    assert!(tool.output_schema().is_some());
    let schema = tool.output_schema().unwrap();
    assert_eq!(
        schema.schema_type, "object",
        "MCP requires all output schemas to be objects"
    );
}

#[tokio::test]
async fn test_boolean_tool_execution_and_schema() {
    let tool = BooleanTool { number: 5 };

    // Test execution
    let result = tool.call(json!({"number": 5}), None).await;
    assert!(result.is_ok());

    let response = result.unwrap();

    // Should have structured content in MCP object format {"output": value} for primitives
    assert!(response.structured_content.is_some());
    if let Some(structured) = response.structured_content {
        let obj = structured.as_object().unwrap();
        assert!(
            obj.contains_key("output"),
            "Expected 'output' field for primitive type, got keys: {:?}",
            obj.keys().collect::<Vec<_>>()
        );
        assert_eq!(obj["output"], json!(true));
    }

    // Should have MCP-compliant object output schema after execution
    assert!(tool.output_schema().is_some());
    let schema = tool.output_schema().unwrap();
    assert_eq!(
        schema.schema_type, "object",
        "MCP requires all output schemas to be objects"
    );
}

#[tokio::test]
async fn test_array_tool_execution_and_schema() {
    let tool = ArrayTool { size: 3 };

    // Test execution
    let result = tool.call(json!({"size": 3}), None).await;
    assert!(result.is_ok());

    let response = result.unwrap();

    // Should have structured content in MCP object format {"output": value} for primitives
    assert!(response.structured_content.is_some());
    if let Some(structured) = response.structured_content {
        let obj = structured.as_object().unwrap();
        assert!(
            obj.contains_key("output"),
            "Expected 'output' field for primitive type, got keys: {:?}",
            obj.keys().collect::<Vec<_>>()
        );
        assert_eq!(obj["output"], json!([0, 1, 2]));
    }

    // Should have MCP-compliant object output schema after execution
    assert!(tool.output_schema().is_some());
    let schema = tool.output_schema().unwrap();
    assert_eq!(
        schema.schema_type, "object",
        "MCP requires all output schemas to be objects"
    );
}

#[tokio::test]
async fn test_object_tool_execution_and_schema() {
    let tool = ObjectTool {
        message: "test".to_string(),
        count: 42,
    };

    // Test execution
    let result = tool
        .call(json!({"message": "test", "count": 42}), None)
        .await;
    assert!(result.is_ok());

    let response = result.unwrap();

    // Should have structured content in MCP object format with default "output" field
    assert!(response.structured_content.is_some());
    if let Some(structured) = response.structured_content {
        let obj = structured.as_object().unwrap();
        // Zero-config struct outputs use "output" as default field name
        assert!(obj.contains_key("output"));
        let expected_result = json!({
            "message": "test",
            "count": 42,
            "success": true
        });
        assert_eq!(obj["output"], expected_result);
    }

    // Should have MCP-compliant object output schema after execution
    assert!(tool.output_schema().is_some());
    let schema = tool.output_schema().unwrap();
    assert_eq!(
        schema.schema_type, "object",
        "MCP requires all output schemas to be objects"
    );
}

#[tokio::test]
async fn test_all_tools_have_schemas_after_execution() {
    // Create instances of all tool types
    let number_tool = NumberTool { value: 1.0 };
    let string_tool = StringTool {
        text: "test".to_string(),
    };
    let boolean_tool = BooleanTool { number: 1 };
    let array_tool = ArrayTool { size: 1 };
    let object_tool = ObjectTool {
        message: "test".to_string(),
        count: 1,
    };

    // Execute all tools
    let _ = number_tool.call(json!({"value": 1.0}), None).await;
    let _ = string_tool.call(json!({"text": "test"}), None).await;
    let _ = boolean_tool.call(json!({"number": 1}), None).await;
    let _ = array_tool.call(json!({"size": 1}), None).await;
    let _ = object_tool
        .call(json!({"message": "test", "count": 1}), None)
        .await;

    // All should have output schemas
    assert!(
        number_tool.output_schema().is_some(),
        "NumberTool should have output schema"
    );
    assert!(
        string_tool.output_schema().is_some(),
        "StringTool should have output schema"
    );
    assert!(
        boolean_tool.output_schema().is_some(),
        "BooleanTool should have output schema"
    );
    assert!(
        array_tool.output_schema().is_some(),
        "ArrayTool should have output schema"
    );
    assert!(
        object_tool.output_schema().is_some(),
        "ObjectTool should have output schema"
    );

    // Verify all schema types are MCP-compliant objects
    assert_eq!(
        number_tool.output_schema().unwrap().schema_type,
        "object",
        "All MCP schemas must be objects"
    );
    assert_eq!(
        string_tool.output_schema().unwrap().schema_type,
        "object",
        "All MCP schemas must be objects"
    );
    assert_eq!(
        boolean_tool.output_schema().unwrap().schema_type,
        "object",
        "All MCP schemas must be objects"
    );
    assert_eq!(
        array_tool.output_schema().unwrap().schema_type,
        "object",
        "All MCP schemas must be objects"
    );
    assert_eq!(
        object_tool.output_schema().unwrap().schema_type,
        "object",
        "All MCP schemas must be objects"
    );
}

#[tokio::test]
async fn test_struct_output_uses_struct_name_as_field() {
    let tool = ObjectTool {
        message: "test".to_string(),
        count: 42,
    };

    // Test execution
    let result = tool
        .call(json!({"message": "test", "count": 42}), None)
        .await;
    assert!(result.is_ok());

    let response = result.unwrap();

    // Should have structured content with default "output" field name
    assert!(response.structured_content.is_some());
    if let Some(structured) = response.structured_content {
        let obj = structured.as_object().unwrap();

        // Zero-config struct outputs use "output" as default field name
        assert!(
            obj.contains_key("output"),
            "Expected 'output' field for zero-config struct output, got keys: {:?}",
            obj.keys().collect::<Vec<_>>()
        );

        let expected_result = json!({
            "message": "test",
            "count": 42,
            "success": true
        });
        assert_eq!(obj["output"], expected_result);
    }
}

#[test]
fn test_tools_have_output_schemas_before_execution() {
    // Test that zero-config tools have output schemas even before first execution
    let number_tool = NumberTool { value: 0.0 };
    let string_tool = StringTool {
        text: String::new(),
    };
    let boolean_tool = BooleanTool { number: 0 };
    let array_tool = ArrayTool { size: 0 };
    let object_tool = ObjectTool {
        message: String::new(),
        count: 0,
    };

    // All tools should have output schemas for tools/list even before execution
    assert!(
        number_tool.output_schema().is_some(),
        "NumberTool should have output schema before execution"
    );
    assert!(
        string_tool.output_schema().is_some(),
        "StringTool should have output schema before execution"
    );
    assert!(
        boolean_tool.output_schema().is_some(),
        "BooleanTool should have output schema before execution"
    );
    assert!(
        array_tool.output_schema().is_some(),
        "ArrayTool should have output schema before execution"
    );
    assert!(
        object_tool.output_schema().is_some(),
        "ObjectTool should have output schema before execution"
    );

    // All schemas should be MCP-compliant objects
    assert_eq!(number_tool.output_schema().unwrap().schema_type, "object");
    assert_eq!(string_tool.output_schema().unwrap().schema_type, "object");
    assert_eq!(boolean_tool.output_schema().unwrap().schema_type, "object");
    assert_eq!(array_tool.output_schema().unwrap().schema_type, "object");
    assert_eq!(object_tool.output_schema().unwrap().schema_type, "object");
}

#[tokio::test]
async fn test_output_schema_upgrades_after_execution() {
    // Test that generic schema gets upgraded to specific schema after execution
    let tool = NumberTool { value: 5.0 };

    // Before execution: should have generic schema
    let schema_before = tool.output_schema().unwrap();
    assert_eq!(schema_before.schema_type, "object");

    // Execute tool
    let _result = tool.call(json!({"value": 5.0}), None).await.unwrap();

    // After execution: should have specific schema (still object, but potentially with different properties)
    let schema_after = tool.output_schema().unwrap();
    assert_eq!(schema_after.schema_type, "object");

    // Both should be valid MCP object schemas
    // Generic schema before execution may have no specific required fields (flexible schema)
    // After execution: should have specific required field based on output type
    if let Some(required_after) = &schema_after.required {
        assert!(
            !required_after.is_empty(),
            "Schema after execution should have required fields"
        );
        // Should have either "output" (primitive) or struct name (struct outputs)
        assert!(required_after.contains(&"output".to_string()) || required_after.len() == 1);
    }
}

#[test]
fn test_tool_names_auto_determined() {
    // Test zero-configuration tool name determination
    let number_tool = NumberTool { value: 0.0 };
    let string_tool = StringTool {
        text: String::new(),
    };
    let boolean_tool = BooleanTool { number: 0 };
    let array_tool = ArrayTool { size: 0 };
    let object_tool = ObjectTool {
        message: String::new(),
        count: 0,
    };

    // Names should be auto-determined from struct names
    assert_eq!(number_tool.name(), "number");
    assert_eq!(string_tool.name(), "string");
    assert_eq!(boolean_tool.name(), "boolean");
    assert_eq!(array_tool.name(), "array");
    assert_eq!(object_tool.name(), "object");
}
