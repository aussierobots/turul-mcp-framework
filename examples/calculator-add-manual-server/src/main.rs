use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use tracing::info;
use turul_mcp_builders::prelude::*; // HasBaseMetadata, HasDescription, etc.
use turul_mcp_protocol::tools::{CallToolResult, ToolAnnotations, ToolResult, ToolSchema};
use turul_mcp_server::{McpResult, McpServer, McpTool, SessionContext};

/// Level 4: Manual Implementation (Maximum Control)
/// This demonstrates the minimal manual approach without any framework helpers.
/// You manually implement all required traits with hardcoded values - no builders, no embedded metadata.
#[derive(Clone)]
struct CalculatorAddTool;

// Minimal manual trait implementations - no helpers, no stored metadata
impl HasBaseMetadata for CalculatorAddTool {
    fn name(&self) -> &str {
        "calculator_add_manual"
    }
    fn title(&self) -> Option<&str> {
        Some("Manual Calculator")
    }
}

impl HasDescription for CalculatorAddTool {
    fn description(&self) -> Option<&str> {
        Some("Add two numbers (Level 4 - Manual Implementation)")
    }
}

impl HasInputSchema for CalculatorAddTool {
    fn input_schema(&self) -> &ToolSchema {
        // Return a static schema - no dynamic construction
        static INPUT_SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        INPUT_SCHEMA.get_or_init(|| {
            use turul_mcp_protocol::schema::JsonSchema;
            ToolSchema::object()
                .with_properties(HashMap::from([
                    ("a".to_string(), JsonSchema::number()),
                    ("b".to_string(), JsonSchema::number()),
                ]))
                .with_required(vec!["a".to_string(), "b".to_string()])
        })
    }
}

impl HasOutputSchema for CalculatorAddTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    } // No schema validation
}

impl HasAnnotations for CalculatorAddTool {
    fn annotations(&self) -> Option<&ToolAnnotations> {
        None
    }
}

impl HasToolMeta for CalculatorAddTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

impl HasIcons for CalculatorAddTool {}

// ToolDefinition automatically implemented via blanket impl!

#[async_trait]
impl McpTool for CalculatorAddTool {
    async fn call(
        &self,
        args: Value,
        _session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        // Manual parameter extraction - no helper methods
        let a = args
            .get("a")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| turul_mcp_protocol::McpError::missing_param("a"))?;

        let b = args
            .get("b")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| turul_mcp_protocol::McpError::missing_param("b"))?;

        // Direct calculation
        let sum = a + b;

        // Simple text response - no structured content or schema validation
        Ok(CallToolResult::success(vec![ToolResult::text(format!(
            "Sum: {}",
            sum
        ))]))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting calculator_add_manual server (Level 4)");
    let server = McpServer::builder()
        .name("calculator_add_manual")
        .version("0.0.1")
        .title("Calculator Add Manual Server")
        .instructions(
            "Add two numbers using fully manual implementation (Level 4 - Maximum Control)",
        )
        .tool(CalculatorAddTool) // No constructor needed
        .bind_address("127.0.0.1:8646".parse()?)
        .build()?;

    server.run().await?;
    Ok(())
}
