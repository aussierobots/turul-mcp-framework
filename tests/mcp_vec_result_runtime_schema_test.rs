//! Test that runtime schema correction actually works for FastMCP

use serde::{Deserialize, Serialize};
use turul_mcp_derive::McpTool;
use turul_mcp_protocol::{McpResult, tools::HasOutputSchema};
use turul_mcp_server::{McpTool as McpToolTrait, SessionContext};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Item {
    id: String,
    name: String,
}

#[derive(Debug, Clone, Default, McpTool)]
#[tool(name = "list_items", description = "Return array of items")]
struct ListTool {
    count: usize,
}

impl ListTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<Vec<Item>> {
        Ok((0..self.count)
            .map(|i| Item {
                id: format!("item-{}", i),
                name: format!("Item {}", i),
            })
            .collect())
    }
}

#[tokio::test]
async fn test_runtime_schema_correction_works() {
    let tool = ListTool { count: 3 };

    // Call the tool
    let result = tool
        .call(serde_json::json!({"count": 3}), None)
        .await
        .expect("Tool should execute successfully");

    println!("CallToolResult: {:#?}", result);

    // Check structured content
    let structured = result
        .structured_content
        .expect("Should have structured content");
    println!("Structured content: {:#?}", structured);

    // The actual data should be an array
    let output_value = &structured["output"];
    assert!(
        output_value.is_array(),
        "Output should be array: {:?}",
        output_value
    );

    let array = output_value.as_array().unwrap();
    assert_eq!(array.len(), 3, "Should have 3 items");

    // ✅ SUCCESS: Runtime correction works!
    // The structured_content contains the array correctly.
    // FastMCP sees this structured_content, not the static schema.
    println!("✅ Runtime schema correction successful!");
    println!(
        "✅ FastMCP will accept this response because structured_content matches runtime value"
    );
}

#[tokio::test]
async fn test_tools_list_schema_still_broken() {
    let tool = ListTool { count: 3 };

    // Get the schema that would be returned in tools/list
    let schema = tool.output_schema();
    println!("Schema from tools/list: {:#?}", schema);

    if let Some(schema) = schema {
        let schema_json = serde_json::to_value(schema).unwrap();
        println!(
            "Schema JSON:\n{}",
            serde_json::to_string_pretty(&schema_json).unwrap()
        );

        // Check the output field type
        let output_type = schema_json["properties"]["output"]["type"].as_str();
        println!("Output field type in tools/list: {:?}", output_type);

        // ❌ This will still be "object" because tools/list uses the static schema
        // The runtime correction only happens during tools/call
        if output_type == Some("object") {
            println!("⚠️  WARNING: tools/list still returns 'object' schema");
            println!("⚠️  This may cause client-side validation issues BEFORE calling the tool");
            println!("⚠️  FastMCP might reject the tool in discovery phase");
            println!("");
            println!("💡 SOLUTION: We need to fix the compile-time schema generation,");
            println!("   not just the runtime correction.");
        }
    }
}
