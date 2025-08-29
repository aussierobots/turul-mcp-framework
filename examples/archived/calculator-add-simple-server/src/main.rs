use mcp_derive::mcp_tool;
use mcp_server::{McpResult, McpServer};
use tracing::info;

/// Level 1: Function Macros (Ultra-Simple)
/// This demonstrates the simplest possible approach using function macros.
/// Compare this ~10 lines to the 100+ lines in calculator-add-manual-server!
#[mcp_tool(
    name = "calculator_add_simple",
    description = "Add two numbers (Level 1 - Function Macro)"
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

    info!("Starting calculator_add_simple server (Level 1)");
    let server = McpServer::builder()
        .name("calculator_add_simple")
        .version("0.0.1")
        .title("Calculator Add Simple Server")
        .instructions("Add two numbers using ultra-simple function macros (Level 1 - Zero Configuration)")
        .tool_fn(calculator_add)  // Framework auto-determines method from function name
        .bind_address("127.0.0.1:8647".parse()?)
        .build()?;

    server.run().await?;
    Ok(())
}