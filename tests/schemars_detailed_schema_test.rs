//! Integration test for detailed schema generation using schemars
//!
//! This test verifies that structs with #[derive(schemars::JsonSchema)]
//! generate detailed output schemas with all fields properly exposed in tools/list

use serde::{Deserialize, Serialize};
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;

/// Accuracy record for testing nested structures
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct AccuracyRecord {
    #[schemars(description = "Timestamp in ISO 8601 format")]
    timestamp: String,

    #[schemars(description = "Horizontal accuracy in meters")]
    h_acc: f32,

    #[schemars(description = "Vertical accuracy in meters")]
    v_acc: f32,

    #[schemars(description = "Longitude coordinate")]
    lon: f64,

    #[schemars(description = "Latitude coordinate")]
    lat: f64,
}

/// Statistics for testing nested objects
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct AccuracyStatistics {
    #[schemars(description = "Minimum horizontal accuracy")]
    h_acc_min: f32,

    #[schemars(description = "Maximum horizontal accuracy")]
    h_acc_max: f32,

    #[schemars(description = "Average horizontal accuracy")]
    h_acc_avg: f32,

    #[schemars(description = "Minimum vertical accuracy")]
    v_acc_min: f32,

    #[schemars(description = "Maximum vertical accuracy")]
    v_acc_max: f32,

    #[schemars(description = "Average vertical accuracy")]
    v_acc_avg: f32,
}

/// Detailed output structure to test schema generation
/// This mimics the user's AccuracyHistoryOutput structure
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct DetailedOutput {
    #[schemars(description = "Device identifier")]
    device_id: String,

    #[schemars(description = "Start time of query range")]
    start_time: String,

    #[schemars(description = "End time of query range")]
    end_time: String,

    #[schemars(description = "Number of records returned")]
    record_count: usize,

    #[schemars(description = "Statistical summary")]
    statistics: AccuracyStatistics,

    #[schemars(description = "Array of accuracy records")]
    accuracy_records: Vec<AccuracyRecord>,
}

/// Tool that uses detailed output
#[derive(McpTool, Clone)]
#[tool(
    name = "get_detailed_data",
    description = "Get detailed data with nested structures",
    output = DetailedOutput
)]
struct DetailedDataTool {
    #[param(description = "Device ID to query")]
    device_id: String,
}

impl DetailedDataTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<DetailedOutput> {
        Ok(DetailedOutput {
            device_id: self.device_id.clone(),
            start_time: "2025-01-01T00:00:00Z".to_string(),
            end_time: "2025-01-02T00:00:00Z".to_string(),
            record_count: 2,
            statistics: AccuracyStatistics {
                h_acc_min: 1.5,
                h_acc_max: 3.2,
                h_acc_avg: 2.35,
                v_acc_min: 2.1,
                v_acc_max: 4.5,
                v_acc_avg: 3.3,
            },
            accuracy_records: vec![
                AccuracyRecord {
                    timestamp: "2025-01-01T12:00:00Z".to_string(),
                    h_acc: 1.5,
                    v_acc: 2.1,
                    lon: -122.4194,
                    lat: 37.7749,
                },
                AccuracyRecord {
                    timestamp: "2025-01-01T18:00:00Z".to_string(),
                    h_acc: 3.2,
                    v_acc: 4.5,
                    lon: -122.4195,
                    lat: 37.7750,
                },
            ],
        })
    }
}

#[tokio::test]
async fn test_detailed_schema_generation() {
    use turul_mcp_builders::prelude::*;

    // Create the tool
    let tool = DetailedDataTool {
        device_id: "test-device".to_string(),
    };

    // Get the output schema
    let schema = tool.output_schema();
    assert!(schema.is_some(), "Tool should have output schema");

    let schema = schema.unwrap();

    // Verify it's an object type
    assert_eq!(schema.schema_type, "object", "Schema should be object type");

    // Verify it has properties
    assert!(schema.properties.is_some(), "Schema should have properties");

    let properties = schema.properties.as_ref().unwrap();

    // Should have the "detailedOutput" field (default output field name)
    assert!(
        properties.contains_key("detailedOutput"),
        "Schema should have 'detailedOutput' field. Found keys: {:?}",
        properties.keys().collect::<Vec<_>>()
    );

    // Get the inner schema
    let inner_schema = &properties["detailedOutput"];

    // Verify it's an object with nested properties (NOT a generic object!)
    if let turul_mcp_protocol::schema::JsonSchema::Object {
        properties: inner_props,
        ..
    } = inner_schema
    {
        assert!(
            inner_props.is_some(),
            "Inner schema should have properties (not generic object). \
             If this fails, schemars conversion is not working!"
        );

        let inner_props = inner_props.as_ref().unwrap();

        // ✅ Check for expected top-level fields
        assert!(
            inner_props.contains_key("device_id"),
            "Should have device_id field"
        );
        assert!(
            inner_props.contains_key("start_time"),
            "Should have start_time field"
        );
        assert!(
            inner_props.contains_key("end_time"),
            "Should have end_time field"
        );
        assert!(
            inner_props.contains_key("record_count"),
            "Should have record_count field"
        );
        assert!(
            inner_props.contains_key("statistics"),
            "Should have statistics field"
        );
        assert!(
            inner_props.contains_key("accuracy_records"),
            "Should have accuracy_records field"
        );

        println!("✓ Top-level fields present in schema");

        // ✅ Verify statistics is a nested object with detailed fields
        if let turul_mcp_protocol::schema::JsonSchema::Object {
            properties: stats_props,
            ..
        } = &inner_props["statistics"]
        {
            assert!(
                stats_props.is_some(),
                "Statistics should have detailed properties, not be generic object!"
            );
            let stats = stats_props.as_ref().unwrap();
            assert!(
                stats.contains_key("h_acc_min"),
                "Statistics should have h_acc_min"
            );
            assert!(
                stats.contains_key("h_acc_max"),
                "Statistics should have h_acc_max"
            );
            assert!(
                stats.contains_key("h_acc_avg"),
                "Statistics should have h_acc_avg"
            );
            assert!(
                stats.contains_key("v_acc_min"),
                "Statistics should have v_acc_min"
            );
            assert!(
                stats.contains_key("v_acc_max"),
                "Statistics should have v_acc_max"
            );
            assert!(
                stats.contains_key("v_acc_avg"),
                "Statistics should have v_acc_avg"
            );
            println!("✓ Nested statistics object has all 6 detail fields");
        } else {
            panic!("Statistics field should be detailed Object with properties, not generic");
        }

        // ✅ Verify accuracy_records is an array of detailed objects
        if let turul_mcp_protocol::schema::JsonSchema::Array { items, .. } =
            &inner_props["accuracy_records"]
        {
            assert!(items.is_some(), "Array should have items schema");

            // Verify the array items are objects with detailed fields
            if let Some(boxed_items) = items {
                if let turul_mcp_protocol::schema::JsonSchema::Object {
                    properties: record_props,
                    ..
                } = boxed_items.as_ref()
                {
                    assert!(
                        record_props.is_some(),
                        "Array items should have detailed properties, not be generic objects!"
                    );
                    let record = record_props.as_ref().unwrap();
                    assert!(
                        record.contains_key("timestamp"),
                        "Record should have timestamp"
                    );
                    assert!(record.contains_key("h_acc"), "Record should have h_acc");
                    assert!(record.contains_key("v_acc"), "Record should have v_acc");
                    assert!(record.contains_key("lon"), "Record should have lon");
                    assert!(record.contains_key("lat"), "Record should have lat");
                    println!("✓ Array items are detailed objects with all 5 fields");
                } else {
                    panic!("Array items should be detailed objects with properties");
                }
            }
        } else {
            panic!("accuracy_records should be an array");
        }

        println!("✅ PASSED: Full nested schema with detailed fields at all levels");
        println!("   - Top-level: 6 fields");
        println!("   - Nested object (statistics): 6 fields");
        println!("   - Array items (accuracy_records): 5 fields each");
    } else {
        panic!(
            "detailedOutput field should be detailed Object with properties, got: {:?}",
            inner_schema
        );
    }
}

#[tokio::test]
async fn test_tool_execution_returns_detailed_data() {
    use serde_json::json;

    let tool = DetailedDataTool {
        device_id: "test-123".to_string(),
    };

    // Execute the tool
    let result = tool.call(json!({"device_id": "test-123"}), None).await;

    assert!(result.is_ok(), "Tool execution should succeed");

    let call_result = result.unwrap();
    assert!(
        call_result.is_error.is_none() || !call_result.is_error.unwrap(),
        "Result should not be an error"
    );

    // Verify we got structured content
    assert!(!call_result.content.is_empty(), "Should have content");

    // The content should include text and potentially structured data
    // Check if we got the device_id back
    let content_str = format!("{:?}", call_result.content);
    assert!(
        content_str.contains("test-123") || content_str.contains("device_id"),
        "Content should reference the device_id"
    );
}
