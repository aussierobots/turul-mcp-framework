//! MCP Tool Schema/Runtime Output Synchronization Tests
//!
//! This test file validates that tools maintain consistency between:
//! 1. The outputSchema reported in tools/list
//! 2. The actual structuredContent returned in tools/call
//!
//! CRITICAL: This validates the MCP 2025-06-18 specification requirement that
//! tools with outputSchema MUST provide matching structuredContent structure.
//!
//! Tests specifically cover the CountAnnouncements scenario where derive macros
//! can generate mismatched schema vs runtime output.

use serde::{Deserialize, Serialize};
use serde_json::json;
use turul_mcp_derive::{McpTool, mcp_tool};
use turul_mcp_protocol::tools::{HasOutputSchema, ToolDefinition};
use turul_mcp_server::{McpResult, McpTool, SessionContext};

/// Test struct that simulates the CountAnnouncements scenario
/// This should demonstrate the schema/runtime mismatch bug
#[derive(Clone, Serialize, Deserialize)]
struct CountAnnouncements {
    pub message: String,
    pub count: u32,
}

#[derive(McpTool, Clone)]
#[tool(
    name = "count_announcements",
    description = "Count announcements with custom output field",
    output = CountAnnouncements,
    output_field = "countResult"
)]
struct CountAnnouncementsTool {
    #[param(description = "Text to analyze")]
    text: String,
}

impl CountAnnouncementsTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<CountAnnouncements> {
        let count = self.text.matches("announcement").count() as u32;
        Ok(CountAnnouncements {
            message: format!("Found {} announcements in the text", count),
            count,
        })
    }
}

/// Test struct for custom output field with primitive type
#[derive(McpTool, Clone)]
#[tool(
    name = "custom_calculator",
    description = "Calculator with custom result field",
    output_field = "calculationResult"
)]
struct CustomCalculatorTool {
    #[param(description = "First number")]
    a: f64,
    #[param(description = "Second number")]
    b: f64,
    #[param(description = "Operation")]
    operation: String,
}

impl CustomCalculatorTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<f64> {
        match self.operation.as_str() {
            "add" => Ok(self.a + self.b),
            "multiply" => Ok(self.a * self.b),
            _ => Err("Unsupported operation".into()),
        }
    }
}

/// Test struct for default output field behavior
#[derive(McpTool, Clone)]
#[tool(
    name = "default_output_tool",
    description = "Tool using default output field"
)]
struct DefaultOutputTool {
    #[param(description = "Input value")]
    value: String,
}

impl DefaultOutputTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok(format!("Processed: {}", self.value))
    }
}

/// Function tool with custom output field for comparison
#[mcp_tool(
    name = "function_with_custom_field",
    description = "Function tool with custom output field",
    output_field = "functionResult"
)]
async fn function_with_custom_field(
    #[param(description = "Input text")] text: String,
) -> McpResult<String> {
    Ok(format!("Function processed: {}", text))
}

#[tokio::test]
async fn test_count_announcements_schema_runtime_sync() {
    let tool = CountAnnouncementsTool {
        text: "This announcement is important. Another announcement follows.".to_string(),
    };

    // Test 1: Check the schema defines the correct output structure
    let schema = tool.output_schema();
    assert!(schema.is_some(), "Tool with output type should have schema");

    if let Some(schema) = schema {
        println!("Schema: {:#?}", schema);

        // Schema should have "countResult" field, not "output"
        if let Some(properties) = &schema.properties {
            assert!(
                properties.contains_key("countResult"),
                "Schema should contain 'countResult' field based on output_field attribute, got properties: {:?}",
                properties.keys().collect::<Vec<_>>()
            );
            assert!(
                !properties.contains_key("output"),
                "Schema should NOT contain 'output' field when custom field is specified"
            );
        }
    }

    // Test 2: Check the runtime output matches the schema
    let args = json!({"text": "This announcement is important. Another announcement follows."});
    let result = tool.call(args, None).await.unwrap();

    // Runtime output must have structuredContent with "countResult" field
    assert!(
        result.structured_content.is_some(),
        "Tool with outputSchema MUST provide structuredContent"
    );

    if let Some(structured) = result.structured_content {
        println!("Runtime structured content: {:#}", structured);

        // CRITICAL: This is where the bug shows up
        // Schema says "countResult" but runtime might return "output" or struct name
        assert!(
            structured.get("countResult").is_some(),
            "Runtime output must use 'countResult' field to match schema, got keys: {:?}",
            structured.as_object().unwrap().keys().collect::<Vec<_>>()
        );

        assert!(
            structured.get("output").is_none(),
            "Runtime should not have 'output' field when custom field is specified"
        );

        // Validate the content structure
        let count_result = structured.get("countResult").unwrap();
        assert!(count_result.is_object(), "countResult should be an object");

        let count_obj = count_result.as_object().unwrap();
        assert!(
            count_obj.contains_key("count"),
            "countResult should have 'count' field"
        );
        assert!(
            count_obj.contains_key("message"),
            "countResult should have 'message' field"
        );
        assert_eq!(count_obj["count"], 2, "Should count 2 announcements");
    }
}

#[tokio::test]
async fn test_custom_calculator_schema_runtime_sync() {
    let tool = CustomCalculatorTool {
        a: 10.0,
        b: 5.0,
        operation: "add".to_string(),
    };

    // Check schema
    let schema = tool.output_schema();
    assert!(schema.is_some(), "Tool should have output schema");

    if let Some(schema) = schema
        && let Some(properties) = &schema.properties
    {
        assert!(
            properties.contains_key("calculationResult"),
            "Schema should contain 'calculationResult' field, got: {:?}",
            properties.keys().collect::<Vec<_>>()
        );
    }

    // Check runtime output
    let args = json!({"a": 10.0, "b": 5.0, "operation": "add"});
    let result = tool.call(args, None).await.unwrap();

    assert!(result.structured_content.is_some());
    if let Some(structured) = result.structured_content {
        assert!(
            structured.get("calculationResult").is_some(),
            "Runtime output must use 'calculationResult' field, got keys: {:?}",
            structured.as_object().unwrap().keys().collect::<Vec<_>>()
        );

        assert_eq!(
            structured["calculationResult"].as_f64().unwrap(),
            15.0,
            "Calculation result should be correct"
        );
    }
}

#[tokio::test]
async fn test_default_output_field_behavior() {
    let tool = DefaultOutputTool {
        value: "test input".to_string(),
    };

    // Check schema uses default "output" field
    let schema = tool.output_schema();
    assert!(schema.is_some());

    if let Some(schema) = schema
        && let Some(properties) = &schema.properties
    {
        // With no custom output_field, should use "output"
        assert!(
            properties.contains_key("output"),
            "Default schema should contain 'output' field, got: {:?}",
            properties.keys().collect::<Vec<_>>()
        );
    }

    // Check runtime output uses default "output" field
    let args = json!({"value": "test input"});
    let result = tool.call(args, None).await.unwrap();

    assert!(result.structured_content.is_some());
    if let Some(structured) = result.structured_content {
        assert!(
            structured.get("output").is_some(),
            "Default runtime output must use 'output' field, got keys: {:?}",
            structured.as_object().unwrap().keys().collect::<Vec<_>>()
        );

        assert_eq!(
            structured["output"].as_str().unwrap(),
            "Processed: test input"
        );
    }
}

#[tokio::test]
async fn test_function_tool_custom_field_sync() {
    let tool = function_with_custom_field();

    // Check schema
    let schema = tool.output_schema();
    assert!(schema.is_some());

    if let Some(schema) = schema
        && let Some(properties) = &schema.properties
    {
        assert!(
            properties.contains_key("functionResult"),
            "Function tool schema should contain 'functionResult' field, got: {:?}",
            properties.keys().collect::<Vec<_>>()
        );
    }

    // Check runtime output
    let args = json!({"text": "function test"});
    let result = tool.call(args, None).await.unwrap();

    assert!(result.structured_content.is_some());
    if let Some(structured) = result.structured_content {
        assert!(
            structured.get("functionResult").is_some(),
            "Function tool runtime must use 'functionResult' field, got keys: {:?}",
            structured.as_object().unwrap().keys().collect::<Vec<_>>()
        );

        assert_eq!(
            structured["functionResult"].as_str().unwrap(),
            "Function processed: function test"
        );
    }
}

#[tokio::test]
async fn test_tools_list_metadata_consistency() {
    // Test that ToolDefinition metadata matches actual tool behavior

    let count_tool = CountAnnouncementsTool {
        text: "test".to_string(),
    };

    let calc_tool = CustomCalculatorTool {
        a: 1.0,
        b: 2.0,
        operation: "add".to_string(),
    };

    // Get tool metadata (what would be returned by tools/list)
    let count_metadata = count_tool.to_tool();
    let calc_metadata = calc_tool.to_tool();

    // Verify metadata output schemas match what we expect
    if let Some(count_schema) = &count_metadata.output_schema
        && let Some(properties) = &count_schema.properties {
            assert!(
                properties.contains_key("countResult"),
                "tools/list metadata should show 'countResult' field"
            );
        }

    if let Some(calc_schema) = &calc_metadata.output_schema
        && let Some(properties) = &calc_schema.properties {
            assert!(
                properties.contains_key("calculationResult"),
                "tools/list metadata should show 'calculationResult' field"
            );
        }

    // Now verify actual tool calls match the metadata
    let count_args = json!({"text": "announcement test"});
    let count_result = count_tool.call(count_args, None).await.unwrap();

    let calc_args = json!({"a": 1.0, "b": 2.0, "operation": "add"});
    let calc_result = calc_tool.call(calc_args, None).await.unwrap();

    // Structured content must match the schema from tools/list
    if let Some(structured) = count_result.structured_content
        && let Some(structured_obj) = structured.as_object() {
            assert!(
                structured_obj.contains_key("countResult"),
                "tools/call output must match tools/list schema field names"
            );
        }

    if let Some(structured) = calc_result.structured_content
        && let Some(structured_obj) = structured.as_object() {
            assert!(
                structured_obj.contains_key("calculationResult"),
                "tools/call output must match tools/list schema field names"
            );
        }
}

/// This test validates the exact scenario you showed with CountAnnouncements
/// where the tools/list shows one schema but tools/call returns different structure
#[tokio::test]
async fn test_specific_count_announcements_scenario() {
    // This recreates the exact issue from your MCP Inspector test
    let tool = CountAnnouncementsTool {
        text: "test text".to_string(),
    };

    // Step 1: Get what tools/list would return
    let tool_definition = tool.to_tool();
    println!("tools/list response for count_announcements:");
    println!("{:#?}", tool_definition);

    // Step 2: Call the tool and see what tools/call returns
    let args = json!({"text": "This has one announcement in it"});
    let call_result = tool.call(args, None).await.unwrap();
    println!("tools/call response for count_announcements:");
    println!("Content: {:?}", call_result.content);
    println!("Structured: {:#?}", call_result.structured_content);

    // Step 3: Validate they match
    // The outputSchema from tools/list should match structuredContent from tools/call
    if let Some(output_schema) = &tool_definition.output_schema
        && let Some(schema_properties) = &output_schema.properties
            && let Some(structured_content) = &call_result.structured_content {
                // Every field in the schema should exist in the structured content
                if let Some(content_obj) = structured_content.as_object() {
                    for schema_field in schema_properties.keys() {
                        assert!(
                            content_obj.contains_key(schema_field),
                            "Schema field '{}' not found in structured content. Schema has: {:?}, Content has: {:?}",
                            schema_field,
                            schema_properties.keys().collect::<Vec<_>>(),
                            content_obj.keys().collect::<Vec<_>>()
                        );
                    }
                }

                // Every field in structured content should be defined in schema
                if let Some(content_obj) = structured_content.as_object() {
                    for content_field in content_obj.keys() {
                        assert!(
                            schema_properties.contains_key(content_field),
                            "Structured content field '{}' not found in schema. Content has: {:?}, Schema has: {:?}",
                            content_field,
                            content_obj.keys().collect::<Vec<_>>(),
                            schema_properties.keys().collect::<Vec<_>>()
                        );
                    }
                }
            }
}

/// Zero-configuration tool without #[tool(...)] attribute - for testing schema/runtime consistency
#[derive(McpTool, Clone, Default)]
struct SimpleCountTool {
    #[param(description = "Text to analyze")]
    text: String,
}

/// Result type for zero-config tool
#[derive(Clone, Serialize, Deserialize)]
struct CountData {
    pub count: u32,
    pub message: String,
}

impl SimpleCountTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<CountData> {
        Ok(CountData {
            count: self.text.len() as u32,
            message: format!("Counted {} characters", self.text.len()),
        })
    }
}

/// Test tool with optional parameter to verify the fix
#[derive(McpTool, Clone)]
#[tool(
    name = "optional_param_test",
    description = "Test tool with optional parameter",
    output = String,
    output_field = "result"
)]
struct OptionalParamTestTool {
    #[param(description = "Required parameter")]
    required_param: String,
    #[param(description = "Optional parameter", optional = true)]
    optional_param: String,
}

impl OptionalParamTestTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok(format!(
            "Required: {}, Optional: {}",
            self.required_param, self.optional_param
        ))
    }
}

#[tokio::test]
async fn test_optional_parameter_bug_fix() {
    // Test that optional parameters are correctly handled in schema and runtime
    let tool = OptionalParamTestTool {
        required_param: "test".to_string(),
        optional_param: "".to_string(), // Default value for optional param
    };

    // Check schema - optional parameter should NOT be in required array
    let tool_def = tool.to_tool();
    let input_schema = &tool_def.input_schema;

    println!("=== Optional Parameter Schema Test ===");
    println!("Required fields: {:?}", input_schema.required);

    // Verify that required_param is required but optional_param is not
    if let Some(required) = &input_schema.required {
        assert!(
            required.contains(&"required_param".to_string()),
            "required_param should be in required array"
        );
        assert!(
            !required.contains(&"optional_param".to_string()),
            "optional_param should NOT be in required array, got: {:?}",
            required
        );
    }

    // Test runtime execution without optional parameter
    let args_without_optional = json!({"required_param": "test_value"});
    let result = tool.call(args_without_optional, None).await;

    println!("Result without optional: {:?}", result);
    assert!(
        result.is_ok(),
        "Tool should execute successfully without optional parameter"
    );

    // Test runtime execution with optional parameter
    let args_with_optional =
        json!({"required_param": "test_value", "optional_param": "optional_value"});
    let result = tool.call(args_with_optional, None).await;

    println!("Result with optional: {:?}", result);
    assert!(
        result.is_ok(),
        "Tool should execute successfully with optional parameter"
    );

    println!("✅ Optional parameter bug fix verified!");
}

#[tokio::test]
async fn test_zero_config_tool_uses_output_field() {
    // This test verifies that zero-config tools (without #[tool(...)] attribute)
    // use "output" field consistently in both schema and runtime
    let tool = SimpleCountTool {
        text: "hello world".to_string(),
    };

    // Test 1: Verify schema uses "output" field
    let schema = tool.output_schema();
    assert!(
        schema.is_some(),
        "Zero-config tool should have output schema"
    );

    if let Some(schema) = schema {
        println!("Zero-config tool schema: {:#?}", schema);

        if let Some(properties) = &schema.properties {
            assert!(
                properties.contains_key("output"),
                "Zero-config tool schema should contain 'output' field, got properties: {:?}",
                properties.keys().collect::<Vec<_>>()
            );
            assert!(
                !properties.contains_key("countData"),
                "Zero-config tool schema should NOT contain camelCase struct name"
            );
        }
    }

    // Test 2: Verify runtime also uses "output" field
    let args = json!({"text": "hello world"});
    let result = tool.call(args, None).await.unwrap();

    assert!(
        result.structured_content.is_some(),
        "Zero-config tool should provide structured content"
    );

    if let Some(structured) = result.structured_content {
        println!("Zero-config tool runtime: {:#}", structured);

        assert!(
            structured.get("output").is_some(),
            "Zero-config tool runtime must use 'output' field to match schema, got keys: {:?}",
            structured.as_object().unwrap().keys().collect::<Vec<_>>()
        );

        assert!(
            structured.get("countData").is_none(),
            "Zero-config tool runtime should NOT have camelCase struct name field"
        );

        // Verify the content structure
        let output_data = structured.get("output").unwrap();
        assert!(output_data.is_object(), "Output should be an object");

        let output_obj = output_data.as_object().unwrap();
        assert!(
            output_obj.contains_key("count"),
            "Output should have 'count' field"
        );
        assert!(
            output_obj.contains_key("message"),
            "Output should have 'message' field"
        );
        assert_eq!(
            output_obj["count"], 11,
            "Should count 11 characters in 'hello world'"
        );
    }

    println!("✅ Zero-config tool schema/runtime consistency verified!");
}
