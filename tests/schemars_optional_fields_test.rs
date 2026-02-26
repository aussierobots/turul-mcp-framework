//! Test schemars with Option<T> fields (may generate anyOf/oneOf patterns)

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use turul_mcp_builders::prelude::*;
use turul_mcp_derive::McpTool;
use turul_mcp_protocol::McpResult;
use turul_mcp_server::{McpTool as McpToolTrait, SessionContext};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct OutputWithOptional {
    #[schemars(description = "Required field")]
    required_field: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Optional field")]
    optional_field: Option<i32>,
}

#[derive(McpTool, Clone)]
#[tool(
    name = "test_optional",
    description = "Test optional fields",
    output = OutputWithOptional
)]
struct OptionalFieldTool {
    #[param(description = "Value to return")]
    value: String,
}

impl OptionalFieldTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<OutputWithOptional> {
        Ok(OutputWithOptional {
            required_field: self.value.clone(),
            optional_field: Some(42),
        })
    }
}

#[tokio::test]
async fn test_optional_fields_schema() {
    let tool = OptionalFieldTool {
        value: "test".to_string(),
    };

    let schema = tool
        .output_schema()
        .expect("Tool should have output schema");

    // Should have a schema (either detailed or fallback)
    assert_eq!(schema.schema_type, "object");

    // Get the output field
    let properties = schema.properties.as_ref().unwrap();
    let inner_schema = &properties["outputWithOptional"];

    match inner_schema {
        turul_mcp_protocol::schema::JsonSchema::Object {
            properties: Some(props),
            ..
        } => {
            // ✅ Detailed schema worked despite Option<T>
            assert!(
                props.contains_key("required_field"),
                "Should have required_field"
            );
            println!("✓ Detailed schema with Option<T> fields");
            println!("  Schema has {} properties", props.len());

            // Optional fields may or may not be in schema depending on schemars behavior
            if props.contains_key("optional_field") {
                println!("  ✓ optional_field included in schema");
            } else {
                println!("  ℹ optional_field not in schema (expected for skip_serializing_if)");
            }
        }
        turul_mcp_protocol::schema::JsonSchema::Object {
            properties: None,
            description,
            ..
        } => {
            // ✅ Safe fallback for complex anyOf/oneOf patterns
            println!("✓ Safe fallback for complex schemars patterns");
            if let Some(desc) = description {
                println!("  Description: {}", desc);
            }
        }
        other => {
            panic!("Unexpected schema type: {:?}", other);
        }
    }

    println!("✅ PASSED: No panics with optional fields");
}

#[tokio::test]
async fn test_optional_field_tool_execution() {
    use serde_json::json;

    let tool = OptionalFieldTool {
        value: "test-value".to_string(),
    };

    // Execute the tool
    let result = tool.call(json!({"value": "test-value"}), None).await;

    assert!(result.is_ok(), "Tool execution should succeed");

    let call_result = result.unwrap();
    assert!(
        call_result.is_error.is_none() || !call_result.is_error.unwrap(),
        "Result should not be an error"
    );

    // Verify we got content
    assert!(!call_result.content.is_empty(), "Should have content");

    println!("✅ PASSED: Tool with optional fields executes successfully");
}
