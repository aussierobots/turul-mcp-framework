//! Test schemars attribute support

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use turul_mcp_derive::McpTool;
use turul_mcp_protocol::McpResult;
use turul_mcp_server::SessionContext;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TestOutput {
    pub value: f64,
    pub message: String,
}

#[derive(McpTool, Default)]
#[tool(
    name = "test_schemars",
    description = "Test schemars integration",
    output = TestOutput
)]
pub struct TestSchemarsTool {
    #[param(description = "Input value")]
    pub input: f64,
}

impl TestSchemarsTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<TestOutput> {
        Ok(TestOutput {
            value: self.input * 2.0,
            message: format!("Doubled: {}", self.input),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use turul_mcp_builders::prelude::*;

    #[tokio::test]
    async fn test_schemars_tool_execution() {
        let tool = TestSchemarsTool { input: 5.0 };
        let result = tool.execute(None).await.unwrap();
        assert_eq!(result.value, 10.0);
        assert_eq!(result.message, "Doubled: 5");
    }

    #[tokio::test]
    async fn test_schemars_generates_detailed_schema() {
        let tool = TestSchemarsTool { input: 0.0 };

        // Get the output schema
        let schema = tool.output_schema();
        assert!(schema.is_some(), "Tool should have output schema");

        let schema = schema.unwrap();

        // Verify it has properties (not generic object!)
        assert!(
            schema.properties.is_some(),
            "Schema must have properties (not just generic object)"
        );

        let properties = schema.properties.as_ref().unwrap();

        // Should have wrapped field (default "result")
        assert!(
            !properties.is_empty(),
            "Schema should have at least one property"
        );

        // Get the wrapped output schema
        let output_schema = properties.values().next().unwrap();

        // CRITICAL: Verify the output schema has detailed properties
        match output_schema {
            turul_mcp_protocol::schema::JsonSchema::Object {
                properties: Some(props),
                ..
            } => {
                // Verify TestOutput fields are present
                assert!(
                    props.contains_key("value"),
                    "Schema should include 'value' field from TestOutput. Got: {:?}",
                    props.keys().collect::<Vec<_>>()
                );
                assert!(
                    props.contains_key("message"),
                    "Schema should include 'message' field from TestOutput. Got: {:?}",
                    props.keys().collect::<Vec<_>>()
                );

                // Verify field types are detailed
                assert!(
                    matches!(
                        &props["value"],
                        turul_mcp_protocol::schema::JsonSchema::Number { .. }
                    ),
                    "'value' should be Number type, not generic object"
                );
                assert!(
                    matches!(
                        &props["message"],
                        turul_mcp_protocol::schema::JsonSchema::String { .. }
                    ),
                    "'message' should be String type, not generic object"
                );

                println!("✓ Schema has detailed field definitions!");
            }
            other => panic!("Expected detailed Object schema, got generic: {:?}", other),
        }
    }
}
