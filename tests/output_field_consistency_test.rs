/// Test that tools/list schema matches tools/call structuredContent
///
/// Bug: When output = Type is specified without explicit output_field,
/// the schema would use camelCase (e.g., "locationLLH") but runtime would use "output"
use serde::{Deserialize, Serialize};
use turul_mcp_derive::McpTool;
use turul_mcp_protocol::tools::HasOutputSchema;
use turul_mcp_server::McpTool as McpToolTrait;

#[derive(Debug, Serialize, Deserialize)]
struct LocationData {
    latitude: f64,
    longitude: f64,
    altitude: f64,
}

// Test case 1: output = Type without explicit output_field
// Should auto-generate "locationData" (camelCase) for BOTH schema and runtime
#[derive(Default, McpTool)]
#[tool(
    name = "get_location",
    description = "Get location data",
    output = LocationData
)]
struct GetLocationTool {}

impl GetLocationTool {
    async fn execute(
        &self,
        _session: Option<turul_mcp_server::SessionContext>,
    ) -> turul_mcp_server::McpResult<LocationData> {
        // Sydney Opera House coordinates
        Ok(LocationData {
            latitude: -33.8568,
            longitude: 151.2153,
            altitude: 5.0,
        })
    }
}

// Test case 2: output = Type WITH explicit output_field
// Should use "LLH" for both schema and runtime
#[derive(Default, McpTool)]
#[tool(
    name = "get_device_location",
    description = "Get device location",
    output = LocationData,
    output_field = "LLH"
)]
struct GetDeviceLocationTool {}

impl GetDeviceLocationTool {
    async fn execute(
        &self,
        _session: Option<turul_mcp_server::SessionContext>,
    ) -> turul_mcp_server::McpResult<LocationData> {
        // Sydney Harbour Bridge coordinates
        Ok(LocationData {
            latitude: -33.8523,
            longitude: 151.2108,
            altitude: 134.0,
        })
    }
}

#[tokio::test]
async fn test_auto_camelcase_consistency() {
    let tool = GetLocationTool::default();

    // Check schema uses "locationData" (camelCase from LocationData)
    let schema = tool.output_schema().expect("Should have output schema");
    let schema_json = serde_json::to_value(&schema).unwrap();
    println!(
        "Schema: {}",
        serde_json::to_string_pretty(&schema_json).unwrap()
    );

    assert!(
        schema_json["properties"]["locationData"].is_object(),
        "Schema should have 'locationData' property (camelCase), got: {}",
        schema_json
    );

    // Execute tool and verify structuredContent also uses "locationData"
    let params = serde_json::json!({});
    let result = tool
        .call(params, None)
        .await
        .expect("Tool should execute successfully");

    if let Some(structured_content) = &result.structured_content {
        println!(
            "Structured Content: {}",
            serde_json::to_string_pretty(structured_content).unwrap()
        );

        assert!(
            structured_content["locationData"].is_object(),
            "structuredContent should have 'locationData' property to match schema, got: {}",
            structured_content
        );

        // Verify the data is correct (Sydney Opera House)
        assert_eq!(
            structured_content["locationData"]["latitude"]
                .as_f64()
                .unwrap(),
            -33.8568
        );
        assert_eq!(
            structured_content["locationData"]["longitude"]
                .as_f64()
                .unwrap(),
            151.2153
        );
    } else {
        panic!("Result should have structured_content");
    }
}

#[tokio::test]
async fn test_explicit_output_field_consistency() {
    let tool = GetDeviceLocationTool::default();

    // Check schema uses "LLH" (explicit output_field)
    let schema = tool.output_schema().expect("Should have output schema");
    let schema_json = serde_json::to_value(&schema).unwrap();
    println!(
        "Schema: {}",
        serde_json::to_string_pretty(&schema_json).unwrap()
    );

    assert!(
        schema_json["properties"]["LLH"].is_object(),
        "Schema should have 'LLH' property (explicit), got: {}",
        schema_json
    );

    // Execute tool and verify structuredContent also uses "LLH"
    let params = serde_json::json!({});
    let result = tool
        .call(params, None)
        .await
        .expect("Tool should execute successfully");

    if let Some(structured_content) = &result.structured_content {
        println!(
            "Structured Content: {}",
            serde_json::to_string_pretty(structured_content).unwrap()
        );

        assert!(
            structured_content["LLH"].is_object(),
            "structuredContent should have 'LLH' property to match schema, got: {}",
            structured_content
        );

        // Verify the data is correct (Sydney Harbour Bridge)
        assert_eq!(
            structured_content["LLH"]["latitude"].as_f64().unwrap(),
            -33.8523
        );
        assert_eq!(
            structured_content["LLH"]["longitude"].as_f64().unwrap(),
            151.2108
        );
    } else {
        panic!("Result should have structured_content");
    }
}

#[tokio::test]
async fn test_schema_validation_would_pass() {
    // This test simulates what MCP Inspector does - validates structuredContent against outputSchema
    let tool = GetDeviceLocationTool::default();

    let schema = tool.output_schema().expect("Should have output schema");
    let schema_json = serde_json::to_value(&schema).unwrap();

    let params = serde_json::json!({});
    let result = tool.call(params, None).await.expect("Tool should execute");

    let structured_content = result
        .structured_content
        .expect("Should have structured content");

    // Get required fields from schema
    let required_fields: Vec<&str> = schema_json["required"]
        .as_array()
        .expect("Schema should have required array")
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();

    println!("Required fields in schema: {:?}", required_fields);
    println!(
        "Fields in structuredContent: {:?}",
        structured_content
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>()
    );

    // Verify all required fields are present in structuredContent
    for field in required_fields {
        assert!(
            structured_content.get(field).is_some(),
            "structuredContent missing required field '{}' from schema. \nSchema: {}\nContent: {}",
            field,
            serde_json::to_string_pretty(&schema_json).unwrap(),
            serde_json::to_string_pretty(&structured_content).unwrap()
        );
    }
}
