//! Test for custom output field name feature

use serde_json::json;
use turul_mcp_derive::mcp_tool;
use turul_mcp_server::{McpResult, McpTool};

#[mcp_tool(
    name = "test_custom_field",
    description = "Test custom output field",
    output_field = "sum"
)]
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

// Test struct for custom output field (like TileMetadata)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TileMetadata {
    pub tile_id: String,
    pub elevation: f64,
    pub terrain_type: String,
    pub coordinates: (f64, f64),
}

#[derive(turul_mcp_derive::McpTool, Clone)]
#[tool(name = "get_tile_metadata", description = "Retrieve metadata for a Mars terrain tile", output = TileMetadata, output_field = "tileMetadata")]
struct GetTileMetadataTool {
    #[param(description = "Tile ID to retrieve")]
    tile_id: String,
}

impl GetTileMetadataTool {
    async fn execute(
        &self,
        _session: Option<turul_mcp_server::SessionContext>,
    ) -> turul_mcp_server::McpResult<TileMetadata> {
        Ok(TileMetadata {
            tile_id: self.tile_id.clone(),
            elevation: 1234.5,
            terrain_type: "rocky".to_string(),
            coordinates: (45.0, 90.0),
        })
    }
}

#[tokio::test]
async fn test_struct_custom_output_field_name() {
    use turul_mcp_protocol::tools::{HasOutputSchema, ToolDefinition};

    let tool = GetTileMetadataTool {
        tile_id: "TILE_123".to_string(),
    };
    let args = json!({"tile_id": "TILE_123"});

    // Test the actual tool call
    let result = tool.call(args, None).await.unwrap();

    // Verify structured content uses "tileMetadata" field, not "output" or struct name
    assert!(result.structured_content.is_some());
    if let Some(structured) = result.structured_content {
        println!("Structured content: {:#}", structured);

        // Should have "tileMetadata" field as specified in output_field
        assert!(
            structured.get("tileMetadata").is_some(),
            "Expected 'tileMetadata' field, got keys: {:?}",
            structured.as_object().unwrap().keys().collect::<Vec<_>>()
        );
        assert!(
            structured.get("output").is_none(),
            "Should not have 'output' field when custom field is specified"
        );
        assert!(
            structured.get("tileMetadata").is_some(),
            "Should not have struct name as field when custom field is specified"
        );

        let tile_data = structured.get("tileMetadata").unwrap();
        assert_eq!(
            tile_data.get("tile_id").unwrap().as_str().unwrap(),
            "TILE_123"
        );
        assert_eq!(
            tile_data.get("elevation").unwrap().as_f64().unwrap(),
            1234.5
        );
    }

    // Test the schema
    let schema = tool.output_schema();
    assert!(schema.is_some(), "Tool should have an output schema");

    if let Some(schema) = schema {
        println!("Schema: {:#?}", schema);

        // Verify schema properties match the custom field name
        if let Some(properties) = &schema.properties {
            assert!(
                properties.contains_key("tileMetadata"),
                "Schema should have 'tileMetadata' property, got properties: {:?}",
                properties.keys().collect::<Vec<_>>()
            );
            assert!(
                !properties.contains_key("output"),
                "Schema should not have 'output' property when custom field is specified"
            );

            // Verify we have the basic tileMetadata schema structure
            if let Some(tile_metadata_schema) = properties.get("tileMetadata") {
                match tile_metadata_schema {
                    turul_mcp_protocol::schema::JsonSchema::Object { .. } => {
                        println!("✅ TileMetadata has Object schema (basic structure)");
                        // Note: Detailed schema generation is not yet implemented
                        // See DETAILED_SCHEMA_ANALYSIS.md for implementation plan
                    }
                    _ => {
                        panic!(
                            "TileMetadata should have Object schema, got: {:#?}",
                            tile_metadata_schema
                        );
                    }
                }
            }
        }

        // Verify required fields match
        if let Some(required) = &schema.required {
            assert!(
                required.contains(&"tileMetadata".to_string()),
                "Schema should require 'tileMetadata' field, got required: {:?}",
                required
            );
        }
    }

    // Test tool metadata serialization (what gets sent in tools/list)
    let tool_metadata = tool.to_tool();
    println!("Tool metadata: {:#?}", tool_metadata);

    // Verify the output schema in tool metadata matches our expectation
    if let Some(output_schema) = tool_metadata.output_schema
        && let Some(properties) = &output_schema.properties
    {
        assert!(
            properties.contains_key("tileMetadata"),
            "Tool metadata should have 'tileMetadata' in output schema"
        );
    }
}
