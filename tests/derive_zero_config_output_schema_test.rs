//! Regression test for zero-config derive macro output schema bug.
//!
//! Previously, `#[derive(McpTool)]` without `#[tool(output = Type)]` would introspect
//! the tool struct itself (Self) as the output type, generating an output schema based
//! on the tool's INPUT fields. This caused MCP Inspector validation errors when the
//! tool's execute() method returned a different type (e.g., String, f64).
//!
//! Fix: zero-config derive tools now return `None` for output_schema(), matching the
//! function macro behavior. Use `#[tool(output = Type)]` for explicit output schemas.

use std::collections::HashMap;
use turul_mcp_builders::prelude::*;
use turul_mcp_derive::McpTool;
use turul_mcp_protocol::McpResult;

// --- Test 1: Zero-config derive tool has NO output schema ---

#[derive(McpTool, Clone, Default)]
#[tool(
    name = "zero_config_tool",
    description = "Tool without explicit output type"
)]
struct ZeroConfigTool {
    a: f64,
    b: f64,
}

impl ZeroConfigTool {
    async fn execute(
        &self,
        _session: Option<turul_mcp_server::SessionContext>,
    ) -> McpResult<String> {
        Ok(format!("{}", self.a + self.b))
    }
}

#[test]
fn test_zero_config_derive_has_no_output_schema() {
    let tool = ZeroConfigTool::default();

    // Zero-config: output_schema must be None (not an incorrect Self-based schema)
    assert!(
        tool.output_schema().is_none(),
        "Zero-config derive tool must NOT generate output schema from Self. \
         Use #[tool(output = Type)] for explicit output schemas."
    );
}

// --- Test 2: Explicit output type still generates schema ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct MyOutput {
    result: f64,
}

impl turul_mcp_protocol::schema::JsonSchemaGenerator for MyOutput {
    fn json_schema() -> turul_mcp_protocol::tools::ToolSchema {
        use turul_mcp_protocol::schema::JsonSchema;
        turul_mcp_protocol::tools::ToolSchema::object()
            .with_properties(HashMap::from([(
                "result".to_string(),
                JsonSchema::number(),
            )]))
            .with_required(vec!["result".to_string()])
    }
}

#[derive(McpTool, Clone, Default)]
#[tool(
    name = "explicit_output_tool",
    description = "Tool with explicit output type",
    output = MyOutput
)]
struct ExplicitOutputTool {
    a: f64,
    b: f64,
}

impl ExplicitOutputTool {
    async fn execute(
        &self,
        _session: Option<turul_mcp_server::SessionContext>,
    ) -> McpResult<MyOutput> {
        Ok(MyOutput {
            result: self.a + self.b,
        })
    }
}

#[test]
fn test_explicit_output_type_generates_schema() {
    let tool = ExplicitOutputTool::default();

    // Explicit output: schema must be present
    assert!(
        tool.output_schema().is_some(),
        "Tool with #[tool(output = MyOutput)] must generate output schema"
    );

    // Verify schema has the expected structure
    let schema = tool.output_schema().unwrap();
    let json = serde_json::to_value(schema).unwrap();

    // Schema wraps MyOutput in a field named after the type (camelCase).
    // Should NOT contain tool input fields (a, b).
    let json_str = json.to_string();
    assert!(
        !json_str.contains("\"a\""),
        "Schema must NOT contain tool input field 'a'. Got: {}",
        json
    );
    assert!(
        !json_str.contains("\"b\""),
        "Schema must NOT contain tool input field 'b'. Got: {}",
        json
    );
    // Should describe MyOutput's "result" field
    assert!(
        json_str.contains("\"result\""),
        "Schema should contain MyOutput's 'result' field. Got: {}",
        json
    );
}
