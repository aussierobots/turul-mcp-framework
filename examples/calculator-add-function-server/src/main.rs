use mcp_derive::mcp_tool;
use mcp_server::{McpResult, McpServer};
use tracing::info;

/// Level 1: Function Tool Macro (#[mcp_tool])
/// Ultra-simple tool definition - just annotate a function
#[mcp_tool(name = "calculator_add_function", description = "Add two numbers using function macro (Level 1)", output_field = "sum")]
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
        .tool_fn(calculator_add) // Perfect! Use the original function name
        .bind_address("127.0.0.1:8648".parse()?)
        .build()?;

    server.run().await?;
    Ok(())
}