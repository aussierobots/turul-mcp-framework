//! Test that #[tool(output = Vec<T>)] generates correct array schemas

use serde::{Deserialize, Serialize};
use turul_mcp_builders::prelude::*;
use turul_mcp_derive::McpTool;
use turul_mcp_builders::prelude::*;
use turul_mcp_protocol::{McpResult, tools::HasOutputSchema};
use turul_mcp_builders::prelude::*;
use turul_mcp_server::{McpTool as McpToolTrait, SessionContext};
use turul_mcp_builders::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataRecord {
    id: u32,
    value: String,
}

/// Tool with explicit Vec<T> output annotation
/// This should generate correct array schema regardless of tool name
#[derive(Debug, Clone, Default, McpTool)]
#[tool(
    name = "fetch_records",
    description = "Fetch data records",
    output = Vec<DataRecord>
)]
struct RecordFetcher {
    limit: usize,
}

impl RecordFetcher {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<Vec<DataRecord>> {
        Ok((0..self.limit)
            .map(|i| DataRecord {
                id: i as u32,
                value: format!("record-{}", i),
            })
            .collect())
    }
}

#[tokio::test]
async fn test_explicit_vec_output_generates_array_schema() {
    let tool = RecordFetcher { limit: 3 };

    // Get the static schema that would be returned in tools/list
    let schema = tool.output_schema().expect("Should have output schema");
    println!("Schema with explicit output = Vec<T>: {:#?}", schema);

    let schema_json = serde_json::to_value(schema).unwrap();
    println!(
        "Schema JSON:\n{}",
        serde_json::to_string_pretty(&schema_json).unwrap()
    );

    // Check the output field type
    let output_type = schema_json["properties"]["output"]["type"].as_str();
    println!("Output field type: {:?}", output_type);

    // ✅ With explicit output = Vec<T>, this SHOULD be "array"
    assert_eq!(
        output_type,
        Some("array"),
        "Explicit #[tool(output = Vec<T>)] should generate array schema, not object"
    );

    println!("✅ Explicit Vec<T> output annotation generates correct array schema!");
}

#[tokio::test]
async fn test_explicit_vec_output_runtime_works() {
    let tool = RecordFetcher { limit: 3 };

    // Call the tool
    let result = tool
        .call(serde_json::json!({"limit": 3}), None)
        .await
        .expect("Tool should execute successfully");

    println!("CallToolResult: {:#?}", result);

    // Check structured content
    let structured = result
        .structured_content
        .expect("Should have structured content");
    let output_value = &structured["output"];

    assert!(
        output_value.is_array(),
        "Output should be array: {:?}",
        output_value
    );

    let array = output_value.as_array().unwrap();
    assert_eq!(array.len(), 3, "Should have 3 records");

    // Verify array contains proper objects
    assert_eq!(array[0]["id"], 0);
    assert_eq!(array[0]["value"], "record-0");

    println!("✅ Explicit Vec<T> output works correctly at runtime!");
}

#[tokio::test]
async fn test_schema_and_runtime_match() {
    let tool = RecordFetcher { limit: 2 };

    // Get schema
    let schema = tool.output_schema().expect("Should have schema");
    let schema_json = serde_json::to_value(schema).unwrap();

    // Get runtime result
    let result = tool
        .call(serde_json::json!({"limit": 2}), None)
        .await
        .unwrap();
    let structured = result
        .structured_content
        .expect("Should have structured content");

    println!(
        "Schema declares: {:#?}",
        schema_json["properties"]["output"]
    );
    println!("Runtime returns: {:#?}", structured["output"]);

    // Both should agree it's an array
    let schema_says_array = schema_json["properties"]["output"]["type"] == "array";
    let runtime_is_array = structured["output"].is_array();

    assert!(schema_says_array, "Schema should declare array type");
    assert!(runtime_is_array, "Runtime should return array value");

    println!("✅ Schema and runtime are synchronized!");
    println!("✅ FastMCP will accept this tool!");
}
