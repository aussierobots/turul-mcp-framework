//! MCP Derive Macro Bug Detection Tests
//!
//! These tests are designed to CATCH the exact bugs shown in MCP Inspector:
//! 1. Schema shows "output" but runtime returns custom field name
//! 2. Optional parameters marked as required in schema
//!
//! CRITICAL: These tests should FAIL until the bugs are fixed.
//! If they pass, the bugs still exist!

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use turul_mcp_derive::McpTool;
use turul_mcp_protocol::tools::ToolDefinition;
use turul_mcp_server::{McpResult, McpTool, SessionContext};

/// This struct reproduces the exact CountAnnouncements scenario
#[derive(Clone, Serialize, Deserialize)]
struct CountResult {
    count: u32,
}

#[derive(McpTool, Clone)]
#[tool(
    name = "count_announcements",
    description = "Count Announcements",
    output = CountResult,
    output_field = "countResult"  // This should make schema use "countResult", not "output"
)]
struct CountAnnouncementsTool {
    #[param(
        description = "Optional ticker to count announcements for (3-4 uppercase letters, e.g. 'BHP')",
        optional = true  // This should make the field optional in schema
    )]
    ticker: String,
}

impl CountAnnouncementsTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<CountResult> {
        // Simulate counting announcements
        Ok(CountResult { count: 622 })
    }
}

#[tokio::test]
async fn test_derive_macro_schema_runtime_mismatch_bug() {
    let tool = CountAnnouncementsTool {
        ticker: "KTA".to_string(),
    };

    // Step 1: Get the schema (what tools/list returns)
    let tool_def = tool.to_tool();
    let output_schema = tool_def.output_schema.as_ref().unwrap();

    println!("=== SCHEMA ANALYSIS ===");
    println!("Schema properties: {:?}", output_schema.properties);

    // Step 2: Call the tool (what tools/call returns)
    let args = json!({"ticker": "KTA"});
    let result = tool.call(args, None).await.unwrap();
    let structured_content = result.structured_content.as_ref().unwrap();

    println!("=== RUNTIME ANALYSIS ===");
    println!("Runtime structured content: {}", structured_content);

    // Step 3: THE BUG CHECK - This should fail until the bug is fixed
    if let Some(schema_props) = &output_schema.properties {
        let runtime_obj = structured_content.as_object().unwrap();

        println!("=== BUG DETECTION ===");
        println!("Schema fields: {:?}", schema_props.keys().collect::<Vec<_>>());
        println!("Runtime fields: {:?}", runtime_obj.keys().collect::<Vec<_>>());

        // BUG: Schema has "output" but runtime has "countResult"
        if schema_props.contains_key("output") && runtime_obj.contains_key("countResult") {
            panic!(
                "🐛 DERIVE MACRO BUG DETECTED! Schema shows 'output' field but runtime uses 'countResult'. \
                 The output_field attribute is not being respected in schema generation!"
            );
        }

        // CORRECT: Schema should have "countResult" to match runtime
        assert!(
            schema_props.contains_key("countResult"),
            "Schema should contain 'countResult' field to match output_field attribute. \
             Got schema fields: {:?}, runtime fields: {:?}",
            schema_props.keys().collect::<Vec<_>>(),
            runtime_obj.keys().collect::<Vec<_>>()
        );

        assert!(
            runtime_obj.contains_key("countResult"),
            "Runtime should contain 'countResult' field as specified by output_field attribute"
        );

        assert!(
            !schema_props.contains_key("output"),
            "Schema should NOT contain 'output' field when custom output_field is specified"
        );
    }
}

#[tokio::test]
async fn test_optional_parameter_schema_bug() {
    let tool = CountAnnouncementsTool {
        ticker: "KTA".to_string(),
    };

    // Get the tool definition (what tools/list returns)
    let tool_def = tool.to_tool();
    let input_schema = &tool_def.input_schema;

    println!("=== INPUT SCHEMA ANALYSIS ===");
    println!("Required fields: {:?}", input_schema.required);
    println!("Properties: {:?}", input_schema.properties);

    // BUG CHECK: ticker is marked as optional but appears in required array
    if let Some(required) = &input_schema.required
        && required.contains(&"ticker".to_string()) {
            panic!(
                "🐛 OPTIONAL PARAMETER BUG DETECTED! 'ticker' is marked with #[param(optional=true)] \
                 but appears in required array: {:?}. Optional parameters should not be required!",
                required
            );
        }

    // CORRECT: Optional parameters should not be in required array
    if let Some(required) = &input_schema.required {
        assert!(
            !required.contains(&"ticker".to_string()),
            "Optional parameter 'ticker' should not be in required array. Required: {:?}",
            required
        );
    }
}

#[tokio::test]
async fn test_mcp_inspector_validation_scenario() {
    // This test simulates exactly what MCP Inspector does
    let tool = CountAnnouncementsTool {
        ticker: "KTA".to_string(),
    };

    // Step 1: Get schema from tools/list
    let tool_def = tool.to_tool();
    let output_schema = tool_def.output_schema.as_ref().unwrap();

    // Step 2: Call tool and get structured content
    let args = json!({"ticker": "KTA"});
    let result = tool.call(args, None).await.unwrap();
    let structured_content = result.structured_content.as_ref().unwrap();

    // Step 3: Validate structured content matches schema (like MCP Inspector does)
    let validation_result = validate_against_schema(structured_content, output_schema);

    if !validation_result.is_valid {
        panic!(
            "🐛 MCP INSPECTOR VALIDATION FAILURE! \
             Schema expects: {:?}, but structured content has: {:?}. \
             Error: {}. \
             This is the exact bug seen in MCP Inspector!",
            output_schema.properties.as_ref().unwrap().keys().collect::<Vec<_>>(),
            structured_content.as_object().unwrap().keys().collect::<Vec<_>>(),
            validation_result.error
        );
    }

    println!("✅ MCP Inspector validation would pass");
}

// Simple schema validation (mimics what MCP Inspector does)
struct ValidationResult {
    is_valid: bool,
    error: String,
}

fn validate_against_schema(content: &Value, schema: &turul_mcp_protocol::tools::ToolSchema) -> ValidationResult {
    if let Some(schema_props) = &schema.properties
        && let Some(required) = &schema.required {
            let content_obj = content.as_object().unwrap();

            // Check all required fields are present
            for required_field in required {
                if !content_obj.contains_key(required_field) {
                    return ValidationResult {
                        is_valid: false,
                        error: format!("data should have required property '{}'", required_field),
                    };
                }
            }
        }

    ValidationResult {
        is_valid: true,
        error: String::new(),
    }
}

/// Test to ensure the fix works correctly
#[tokio::test]
async fn test_correct_behavior_after_fix() {
    // This test should pass AFTER the bugs are fixed
    let tool = CountAnnouncementsTool {
        ticker: "KTA".to_string(),
    };

    // Schema and runtime should match exactly
    let tool_def = tool.to_tool();
    let output_schema = tool_def.output_schema.as_ref().unwrap();

    let args = json!({"ticker": "KTA"});
    let result = tool.call(args, None).await.unwrap();
    let structured_content = result.structured_content.as_ref().unwrap();

    // After fix: schema should have "countResult"
    if let Some(schema_props) = &output_schema.properties {
        assert!(
            schema_props.contains_key("countResult"),
            "Fixed schema should contain 'countResult' field"
        );
    }

    // After fix: runtime should have "countResult"
    let content_obj = structured_content.as_object().unwrap();
    assert!(
        content_obj.contains_key("countResult"),
        "Runtime should contain 'countResult' field"
    );

    // After fix: MCP Inspector validation should pass
    let validation = validate_against_schema(structured_content, output_schema);
    assert!(validation.is_valid, "Validation should pass after fix: {}", validation.error);

    println!("✅ All validations pass - bug is fixed!");
}