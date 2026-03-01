// turul-mcp-server v0.3
// Level 3: Builder Pattern (ToolBuilder) — runtime flexibility
//
// Source: examples/calculator-add-builder-server/src/main.rs

use serde_json::json;
use tracing::info;
use turul_mcp_server::{McpServer, ToolBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting calculator_add_builder server");

    // Build tool at runtime using builder pattern.
    // Parameters are defined with typed helpers.
    // Manual extraction in the execute closure (no compile-time type safety).
    let add_tool = ToolBuilder::new("calculator_add_builder")
        .description("Add two numbers using builder pattern (Level 3)")
        .number_param("a", "First number")
        .number_param("b", "Second number")
        .number_output() // Generates {"result": number} schema
        .execute(|args| async move {
            let a = args
                .get("a")
                .and_then(|v| v.as_f64()) // Always use as_f64() for numbers
                .ok_or("Missing or invalid parameter 'a'")?;
            let b = args
                .get("b")
                .and_then(|v| v.as_f64())
                .ok_or("Missing or invalid parameter 'b'")?;

            let sum = a + b;
            Ok(json!({"result": sum}))
        })
        .build()
        .map_err(|e| format!("Failed to build tool: {}", e))?;

    let server = McpServer::builder()
        .name("calculator_add_builder")
        .version("0.0.1")
        .title("Calculator Add Builder Server")
        .instructions("Add two numbers using builder pattern (Level 3 - Runtime Flexibility)")
        .tool(add_tool) // Use .tool() for builder tools (same as derive)
        .build()?;

    server.run().await?;
    Ok(())
}
