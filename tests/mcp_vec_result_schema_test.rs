//! Test for Vec<T> return type schema generation bug
//!
//! **Issue**: Tools returning Vec<T> generate incorrect schemas:
//! - Current: Wraps in object with "output" field containing object type
//! - Expected: Should be array type at top level or correctly typed array in output field
//!
//! **Root Cause**: tool_derive.rs always wraps return values in ToolSchema::object()
//! even when the actual return type is Vec<T> (array)
//!
//! **Impact**: FastMCP and MCP Inspector schema validation rejects valid array responses

use serde::{Deserialize, Serialize};
use turul_mcp_builders::prelude::*;
use turul_mcp_derive::McpTool;
use turul_mcp_builders::prelude::*;
use turul_mcp_protocol::{McpResult, tools::HasOutputSchema};
use turul_mcp_builders::prelude::*;
use turul_mcp_server::{McpTool as McpToolTrait, SessionContext};
use turul_mcp_builders::prelude::*;

/// Test struct for array item
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SearchResult {
    id: String,
    title: String,
    score: f64,
}

/// Tool that returns Vec<SearchResult> - this causes schema validation issues
#[derive(Debug, Clone, Default, McpTool)]
#[tool(
    name = "search_items",
    description = "Search for items and return array of results"
)]
struct SearchTool {
    query: String,
    #[allow(dead_code)]
    limit: usize,
}

impl SearchTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<Vec<SearchResult>> {
        // Simulate search results
        Ok(vec![
            SearchResult {
                id: "1".to_string(),
                title: format!("Result for: {}", self.query),
                score: 0.95,
            },
            SearchResult {
                id: "2".to_string(),
                title: format!("Another result for: {}", self.query),
                score: 0.87,
            },
        ])
    }
}

#[tokio::test]
async fn test_vec_result_schema_generation() {
    let tool = SearchTool {
        query: "test query".to_string(),
        limit: 10,
    };

    // 1. Check the output schema
    let schema = tool.output_schema();
    assert!(
        schema.is_some(),
        "Tool with Vec<T> return should have output schema"
    );

    let schema = schema.unwrap();
    println!("Generated schema: {:#?}", schema);

    // Serialize to JSON for inspection
    let schema_json = serde_json::to_value(schema).expect("Schema should serialize");
    println!(
        "Schema JSON:\n{}",
        serde_json::to_string_pretty(&schema_json).unwrap()
    );

    // 2. The BUG: Check if schema type is correctly identified
    let schema_type = schema_json["type"].as_str();
    println!("Top-level schema type: {:?}", schema_type);

    // Current behavior: type = "object" with properties.output = object (WRONG)
    // Expected behavior: Either:
    //   - type = "object" with properties.output = array (CORRECT)
    //   - OR type = "array" at top level (ALSO CORRECT)

    if schema_type == Some("object") {
        // If wrapped in object, check the output field
        let output_field = &schema_json["properties"]["output"];
        println!("Output field schema: {:#?}", output_field);

        let output_type = output_field["type"].as_str();
        println!("Output field type: {:?}", output_type);

        // ❌ BUG: This currently fails because output_type = "object" not "array"
        assert_eq!(
            output_type,
            Some("array"),
            "❌ BUG DETECTED: Vec<T> return generates 'object' schema instead of 'array'. \
             FastMCP/MCP Inspector will reject this as invalid."
        );
    } else if schema_type == Some("array") {
        // Alternative valid approach: top-level array schema
        println!("✅ Schema correctly identifies return type as array");
    } else {
        panic!(
            "❌ Unexpected schema type: {:?}. Expected 'object' with array field or 'array' at top level",
            schema_type
        );
    }
}

#[tokio::test]
async fn test_vec_result_actual_return_value() {
    let tool = SearchTool {
        query: "test".to_string(),
        limit: 2,
    };

    // Execute the tool with proper parameters
    let result = tool
        .call(
            serde_json::json!({
                "query": "test",
                "limit": 2
            }),
            None,
        )
        .await;
    assert!(result.is_ok(), "Tool execution should succeed");

    let result = result.unwrap();
    println!("Tool result: {:#?}", result);

    // Check if result has content
    assert!(!result.content.is_empty(), "Tool should return content");

    // Parse the returned content
    let content_text = match &result.content[0] {
        turul_mcp_protocol::ContentBlock::Text { text, .. } => text,
        turul_mcp_protocol::ContentBlock::Image { .. } => panic!("Expected text content"),
        turul_mcp_protocol::ContentBlock::Resource { .. } => panic!("Expected text content"),
        turul_mcp_protocol::ContentBlock::Audio { .. } => panic!("Expected text content"),
        turul_mcp_protocol::ContentBlock::ResourceLink { .. } => panic!("Expected text content"),
    };
    let content_json: serde_json::Value =
        serde_json::from_str(content_text).expect("Content should be valid JSON");
    println!(
        "Content JSON:\n{}",
        serde_json::to_string_pretty(&content_json).unwrap()
    );

    // Check if structured content exists (should for tools with output schema)
    if let Some(structured) = &result.structured_content {
        println!("Structured content: {:#?}", structured);

        // The structured content should contain the array under "output" field
        if let Some(output_field) = structured.get("output") {
            println!("Output field value: {:#?}", output_field);

            // ✅ The actual data IS an array
            assert!(
                output_field.is_array(),
                "Output field should contain array data"
            );

            let array = output_field.as_array().unwrap();
            assert_eq!(array.len(), 2, "Should have 2 search results");
        } else {
            panic!("❌ Structured content missing 'output' field");
        }
    } else {
        panic!("❌ Tool with output schema must provide structured_content");
    }
}

#[tokio::test]
async fn test_schema_validation_would_pass() {
    let tool = SearchTool {
        query: "test".to_string(),
        limit: 2,
    };

    // Get schema and actual result
    let schema = tool.output_schema().expect("Should have schema");
    let result = tool
        .call(
            serde_json::json!({
                "query": "test",
                "limit": 2
            }),
            None,
        )
        .await
        .unwrap();

    let schema_json = serde_json::to_value(schema).unwrap();
    let structured_content = result
        .structured_content
        .expect("Should have structured content");

    println!("Schema declares: {:#?}", schema_json);
    println!("Runtime returns: {:#?}", structured_content);

    // Simulate FastMCP validation logic
    // FastMCP will check: Does structured_content["output"] match schema["properties"]["output"]?

    let schema_output_type = schema_json["properties"]["output"]["type"].as_str();
    let actual_output_value = &structured_content["output"];

    println!("Schema says output type: {:?}", schema_output_type);
    println!(
        "Runtime output is_array: {}",
        actual_output_value.is_array()
    );
    println!(
        "Runtime output is_object: {}",
        actual_output_value.is_object()
    );

    // ❌ THE MISMATCH:
    // - Schema says: "output": { "type": "object", "additionalProperties": true }
    // - Runtime returns: "output": [ {...}, {...} ]  (array)
    // - FastMCP: "Type mismatch! Expected object, got array" → REJECT

    if schema_output_type == Some("object") && actual_output_value.is_array() {
        panic!(
            "❌ SCHEMA/RUNTIME MISMATCH DETECTED!\n\
             Schema declares: output type = 'object'\n\
             Runtime returns: output value = array\n\
             This causes FastMCP schema validation to REJECT the response.\n\
             \n\
             FIX NEEDED: tool_derive.rs should detect Vec<T> returns and generate:\n\
             \"output\": {{ \"type\": \"array\", \"items\": {{...}} }}"
        );
    }
}
