//! # Truly Minimal MCP Server
//!
//! This is the simplest possible MCP server - just one function with the #[mcp_tool] attribute.
//! Perfect for getting started and understanding the basics.

use turul_mcp_derive::mcp_tool;
use turul_mcp_server::prelude::*;

/// The simplest possible MCP tool - just echo back the input text
#[mcp_tool(name = "echo", description = "Echo back the input text")]
async fn echo(#[param(description = "Text to echo back")] text: String) -> McpResult<String> {
    Ok(format!("Echo: {}", text))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize basic logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("Starting Truly Minimal MCP Server");

    // Create the simplest possible MCP server - just name, version, and one function tool
    let server = McpServer::builder()
        .name("minimal-server") // Required: Server name
        .version("1.0.0") // Required: Server version
        .tool_fn(echo) // Use function name directly
        .bind_address("127.0.0.1:8641".parse()?)
        .build()?;

    println!("Minimal MCP server running at: http://127.0.0.1:8641/mcp");
    println!("This server has ONE tool: echo");

    server.run().await?;
    Ok(())
}
