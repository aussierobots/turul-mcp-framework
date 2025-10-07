/// Test that acronym type names are correctly converted to camelCase
/// Bug fix: LLH → llh (not lLH), GPS → gps (not gPS)
use serde::{Deserialize, Serialize};
use turul_mcp_builders::prelude::*;
use turul_mcp_derive::McpTool;
use turul_mcp_builders::prelude::*;
use turul_mcp_protocol::tools::HasOutputSchema;
use turul_mcp_builders::prelude::*;
use turul_mcp_server::McpTool as McpToolTrait;
use turul_mcp_builders::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
struct LLH {
    latitude: f64,
    longitude: f64,
    height: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
struct GPS {
    x: f64,
    y: f64,
    z: f64,
}

// Test case 1: All-caps acronym LLH should become "llh" (not "lLH")
#[derive(Default, McpTool)]
#[tool(
    name = "get_llh_position",
    description = "Get LLH position",
    output = LLH
)]
struct GetLLHPositionTool {}

impl GetLLHPositionTool {
    async fn execute(
        &self,
        _session: Option<turul_mcp_server::SessionContext>,
    ) -> turul_mcp_server::McpResult<LLH> {
        // Uluru coordinates
        Ok(LLH {
            latitude: -25.3444,
            longitude: 131.0369,
            height: 863.0,
        })
    }
}

// Test case 2: All-caps acronym GPS should become "gps" (not "gPS")
#[derive(Default, McpTool)]
#[tool(
    name = "get_gps_coordinates",
    description = "Get GPS coordinates",
    output = GPS
)]
struct GetGPSCoordinatesTool {}

impl GetGPSCoordinatesTool {
    async fn execute(
        &self,
        _session: Option<turul_mcp_server::SessionContext>,
    ) -> turul_mcp_server::McpResult<GPS> {
        Ok(GPS {
            x: 1000.0,
            y: 2000.0,
            z: 3000.0,
        })
    }
}

#[tokio::test]
async fn test_llh_acronym_to_lowercase() {
    let tool = GetLLHPositionTool::default();

    // Check schema uses "llh" (all lowercase, not "lLH")
    let schema = tool.output_schema().expect("Should have output schema");
    let schema_json = serde_json::to_value(schema).unwrap();
    println!(
        "LLH Schema: {}",
        serde_json::to_string_pretty(&schema_json).unwrap()
    );

    assert!(
        schema_json["properties"]["llh"].is_object(),
        "Schema should have 'llh' property (all lowercase), got: {}",
        schema_json
    );

    // Verify "lLH" (the bug) doesn't exist
    assert!(
        schema_json["properties"]["lLH"].is_null(),
        "Schema should NOT have 'lLH' property (that's the bug!)"
    );

    // Execute tool and verify structuredContent also uses "llh"
    let params = serde_json::json!({});
    let result = tool
        .call(params, None)
        .await
        .expect("Tool should execute successfully");

    if let Some(structured_content) = &result.structured_content {
        println!(
            "LLH Structured Content: {}",
            serde_json::to_string_pretty(structured_content).unwrap()
        );

        assert!(
            structured_content["llh"].is_object(),
            "structuredContent should have 'llh' property to match schema, got: {}",
            structured_content
        );

        // Verify the bug doesn't exist
        assert!(
            structured_content["lLH"].is_null(),
            "structuredContent should NOT have 'lLH' (that would be the bug)"
        );
    } else {
        panic!("Result should have structured_content");
    }
}

#[tokio::test]
async fn test_gps_acronym_to_lowercase() {
    let tool = GetGPSCoordinatesTool::default();

    // Check schema uses "gps" (all lowercase, not "gPS")
    let schema = tool.output_schema().expect("Should have output schema");
    let schema_json = serde_json::to_value(schema).unwrap();
    println!(
        "GPS Schema: {}",
        serde_json::to_string_pretty(&schema_json).unwrap()
    );

    assert!(
        schema_json["properties"]["gps"].is_object(),
        "Schema should have 'gps' property (all lowercase), got: {}",
        schema_json
    );

    // Verify "gPS" (the bug) doesn't exist
    assert!(
        schema_json["properties"]["gPS"].is_null(),
        "Schema should NOT have 'gPS' property (that's the bug!)"
    );

    // Execute tool and verify structuredContent also uses "gps"
    let params = serde_json::json!({});
    let result = tool
        .call(params, None)
        .await
        .expect("Tool should execute successfully");

    if let Some(structured_content) = &result.structured_content {
        println!(
            "GPS Structured Content: {}",
            serde_json::to_string_pretty(structured_content).unwrap()
        );

        assert!(
            structured_content["gps"].is_object(),
            "structuredContent should have 'gps' property to match schema, got: {}",
            structured_content
        );

        // Verify the bug doesn't exist
        assert!(
            structured_content["gPS"].is_null(),
            "structuredContent should NOT have 'gPS' (that would be the bug)"
        );
    } else {
        panic!("Result should have structured_content");
    }
}
