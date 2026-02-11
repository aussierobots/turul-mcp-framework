//! Test that runtime schema correction works even for badly-named tools
//! that return Vec<T> but don't match heuristics

use serde::{Deserialize, Serialize};
use turul_mcp_builders::prelude::HasOutputSchema;
use turul_mcp_derive::McpTool;
use turul_mcp_protocol::McpResult;
use turul_mcp_server::{McpTool as McpToolTrait, SessionContext};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Record {
    id: u32,
    value: String,
}

/// This tool name doesn't match any heuristics (not Search, List, Array, Query, Find, Batch)
/// but it still returns Vec<Record>
#[derive(Debug, Clone, Default, McpTool)]
#[tool(name = "fetch_data", description = "Fetch some data")]
struct DataFetcher {
    count: usize,
}

impl DataFetcher {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<Vec<Record>> {
        Ok((0..self.count)
            .map(|i| Record {
                id: i as u32,
                value: format!("record-{}", i),
            })
            .collect())
    }
}

#[tokio::test]
async fn test_runtime_correction_for_badly_named_tool() {
    let tool = DataFetcher { count: 5 };

    // Call the tool
    let result = tool
        .call(serde_json::json!({"count": 5}), None)
        .await
        .expect("Tool should execute successfully");

    println!("CallToolResult: {:#?}", result);

    // Check structured content
    let structured = result
        .structured_content
        .expect("Should have structured content");
    println!("Structured content: {:#?}", structured);

    let output_value = &structured["output"];
    println!("Output value: {:#?}", output_value);
    println!("Is array: {}", output_value.is_array());
    println!("Is object: {}", output_value.is_object());

    // ✅ The runtime correction should have fixed this
    assert!(
        output_value.is_array(),
        "Runtime correction should convert object schema to array for Vec<T> returns"
    );

    let array = output_value.as_array().unwrap();
    assert_eq!(array.len(), 5, "Should have 5 records");

    println!("✅ Runtime correction works even for badly-named tools!");
}

#[tokio::test]
async fn test_badly_named_tool_static_schema_is_wrong() {
    let tool = DataFetcher { count: 5 };

    // Get the static schema
    let schema = tool.output_schema();
    println!("Static schema: {:#?}", schema);

    if let Some(schema) = schema {
        let schema_json = serde_json::to_value(schema).unwrap();
        let output_type = schema_json["properties"]["output"]["type"].as_str();
        println!("Static schema output type: {:?}", output_type);

        // ❌ The static schema will be wrong because "DataFetcher" doesn't match heuristics
        if output_type == Some("object") {
            println!("⚠️  Static schema is WRONG (says 'object' not 'array')");
            println!("✅ But runtime correction fixes it during tools/call");
            println!("📝 For 0.3.0: Consider #[tool(returns = Vec<T>)] attribute");
        } else if output_type == Some("array") {
            println!("✅ Static schema is correct (either heuristic matched or corrected)");
        }
    }
}
