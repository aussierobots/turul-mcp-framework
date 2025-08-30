//! # Macro Calculator Server Example
//!
//! This example demonstrates the MCP derive macros for creating tools with
//! significantly less boilerplate code.

use std::net::SocketAddr;

use turul_mcp_derive::McpTool;
use turul_mcp_server::{McpResult, McpServer};
use tracing::info;

/// Addition tool using derive macro
#[derive(McpTool, Clone)]
#[tool(name = "add", description = "Add two numbers together")]
struct AddTool {
    #[param(description = "First number")]
    a: f64,
    #[param(description = "Second number")]
    b: f64,
}

impl AddTool {
    async fn execute(&self) -> McpResult<String> {
        Ok(format!("{} + {} = {}", self.a, self.b, self.a + self.b))
    }
}

/// Subtraction tool using derive macro
#[derive(McpTool, Clone)]
#[tool(name = "subtract", description = "Subtract second number from first")]
struct SubtractTool {
    #[param(description = "First number")]
    a: f64,
    #[param(description = "Second number")]
    b: f64,
}

impl SubtractTool {
    async fn execute(&self) -> McpResult<String> {
        Ok(format!("{} - {} = {}", self.a, self.b, self.a - self.b))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt().init();

    info!("Starting Macro Calculator MCP Server");

    // Parse command line arguments for bind address
    let bind_address: SocketAddr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8765".to_string())
        .parse()
        .map_err(|e| format!("Invalid bind address: {}", e))?;

    // Build the MCP server with macro-generated tools
    let server = McpServer::builder()
        .name("macro-calculator-server")
        .version("1.0.0")
        .title("Macro Calculator MCP Server")
        .instructions("This server provides calculator operations using MCP derive macros. Tools: add, subtract.")
        .tool(AddTool { a: 0.0, b: 0.0 })  // Dummy instance for derive
        .tool(SubtractTool { a: 0.0, b: 0.0 })  // Dummy instance for derive
        .bind_address(bind_address)
        .mcp_path("/mcp")
        .cors(true)
        .sse(true)
        .build()?;

    info!("Macro calculator server configured");
    info!("Server will bind to: {}", bind_address);
    info!("MCP endpoint available at: http://{}/mcp", bind_address);
    info!("Tools available: add (derive), subtract (derive)");

    // Run the server
    server.run().await?;

    Ok(())
}
