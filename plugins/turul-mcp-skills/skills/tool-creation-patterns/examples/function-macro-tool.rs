// turul-mcp-server v0.3
// Level 1: Function Macro (#[mcp_tool]) — simplest tool pattern
//
// Source: examples/calculator-add-function-server/src/main.rs

use tracing::info;
use turul_mcp_derive::mcp_tool;
use turul_mcp_server::{McpResult, McpServer};

/// Annotate an async function to create a tool.
/// Parameters are extracted from the function signature automatically.
#[mcp_tool(
    name = "calculator_add_function",
    description = "Add two numbers using function macro (Level 1)",
    output_field = "sum"
)]
async fn calculator_add(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<f64> {
    Ok(a + b)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting calculator_add_function server");

    let server = McpServer::builder()
        .name("calculator_add_function")
        .version("0.0.1")
        .title("Calculator Add Function Server")
        .instructions("Add two numbers using function macro (Level 1 - Ultra Simple)")
        .tool_fn(calculator_add) // Use .tool_fn() for function macros
        .build()?;

    server.run().await?;
    Ok(())
}
