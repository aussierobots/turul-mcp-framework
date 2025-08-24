use async_trait::async_trait;
use mcp_protocol::{CallToolResponse, ToolResult, ToolSchema};
use mcp_server::{McpResult, McpServer, McpTool, SessionContext};
// use serde::Serialize; // Not used in this simple example
use serde_json::Value;
use tracing::info;

/// Manual implementation of a calculator tool without derive macros
/// This implementation extracts parameters directly in the execute() method using helper functions
#[derive(Clone)]
struct CalculatorAddTool {
    // Manual tools typically don't need fields unless they maintain state
}

impl CalculatorAddTool {
    fn new() -> Self {
        Self {}
    }

    async fn calculate(&self, a: f64, b: f64) -> McpResult<f64> {
        Ok(a + b)
    }

    fn extract_param_a(&self, args: &Value) -> McpResult<f64> {
        args.get("a")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| mcp_protocol::McpError::invalid_param_type("a", "number", "other"))
    }

    fn extract_param_b(&self, args: &Value) -> McpResult<f64> {
        args.get("b")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| mcp_protocol::McpError::invalid_param_type("b", "number", "other"))
    }
}

#[async_trait]
impl McpTool for CalculatorAddTool {
    fn name(&self) -> &str {
        "calculator_add_manual"
    }

    fn description(&self) -> &str {
        "Add two numbers (manual implementation)"
    }

    fn input_schema(&self) -> ToolSchema {
        use mcp_protocol::schema::JsonSchema;
        use std::collections::HashMap;

        ToolSchema::object()
            .with_properties(HashMap::from([
                (
                    "a".to_string(),
                    JsonSchema::number().with_description("First number"),
                ),
                (
                    "b".to_string(),
                    JsonSchema::number().with_description("Second number"),
                ),
            ]))
            .with_required(vec!["a".to_string(), "b".to_string()])
    }

    fn output_schema(&self) -> Option<ToolSchema> {
        use mcp_protocol::schema::JsonSchema;
        use std::collections::HashMap;

        Some(
            ToolSchema::object()
                .with_properties(HashMap::from([("sum".to_string(), JsonSchema::number())]))
                .with_required(vec!["sum".to_string()]),
        )
    }

    async fn call(
        &self,
        args: Value,
        session: Option<SessionContext>,
    ) -> McpResult<Vec<ToolResult>> {
        // Delegate to execute() and extract content
        let response = self.execute(args, session).await?;
        Ok(response.content)
    }

    async fn execute(
        &self,
        args: Value,
        _session: Option<SessionContext>,
    ) -> McpResult<CallToolResponse> {
        // Extract parameters directly
        let a = self.extract_param_a(&args)?;
        let b = self.extract_param_b(&args)?;

        // Execute calculation
        let result = self.calculate(a, b).await?;

        // Create text content
        let json_text = serde_json::json!({"sum": result}).to_string();
        let content = vec![ToolResult::text(json_text)];
        let response = CallToolResponse::success(content);

        // Add structured content since we have output schema
        if self.output_schema().is_some() {
            let structured_content = serde_json::json!({"sum": result});
            Ok(response.with_structured_content(structured_content))
        } else {
            Ok(response)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting calculator_add_manual server");
    let server = McpServer::builder()
        .name("calculator_add_manual")
        .version("0.0.1")
        .title("Calculator Add Manual Server")
        .instructions("Add two numbers using manual trait implementation")
        .tool(CalculatorAddTool::new())
        .bind_address("127.0.0.1:8646".parse()?)
        .build()?;

    server.run().await?;
    Ok(())
}
