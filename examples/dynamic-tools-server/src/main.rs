//! # Dynamic Tools Server
//!
//! Demonstrates runtime tool activation/deactivation with MCP-compliant
//! `notifications/tools/list_changed` notifications.
//!
//! ## Testing with curl (two terminals)
//!
//! Terminal 1 — Start server:
//!   cargo run -p dynamic-tools-server
//!
//! Terminal 2 — Initialize session:
//!   curl -si -X POST http://127.0.0.1:8484/mcp \
//!     -H "Content-Type: application/json" -H "Accept: application/json" \
//!     -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
//!   # Copy the Mcp-Session-Id from the response headers
//!
//! Terminal 2 — Complete handshake:
//!   curl -s -X POST http://127.0.0.1:8484/mcp \
//!     -H "Content-Type: application/json" -H "Accept: application/json" \
//!     -H "Mcp-Session-Id: <ID>" \
//!     -d '{"jsonrpc":"2.0","method":"notifications/initialized"}'
//!
//! Terminal 2 — Open SSE stream for server notifications:
//!   curl -N http://127.0.0.1:8484/mcp \
//!     -H "Accept: text/event-stream" \
//!     -H "Mcp-Session-Id: <ID>"
//!
//! Wait 15 seconds — 'multiply' will be deactivated.
//! You should see on the SSE stream:
//!   data: {"method":"notifications/tools/list_changed","jsonrpc":"2.0"}
//!
//! Terminal 3 — Verify tools/list changed:
//!   curl -s -X POST http://127.0.0.1:8484/mcp \
//!     -H "Content-Type: application/json" -H "Accept: application/json" \
//!     -H "Mcp-Session-Id: <ID>" \
//!     -d '{"jsonrpc":"2.0","id":2,"method":"tools/list"}'
//!   # Should show only 'add' and 'greet', no 'multiply'

use tracing::info;
use turul_mcp_derive::McpTool;
use turul_mcp_server::{McpResult, McpServer, SessionContext, ToolChangeMode};

#[derive(McpTool, Clone, Default)]
#[tool(name = "add", description = "Add two numbers")]
struct AddTool {
    a: f64,
    b: f64,
}

impl AddTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok(format!("{}", self.a + self.b))
    }
}

#[derive(McpTool, Clone, Default)]
#[tool(name = "multiply", description = "Multiply two numbers")]
struct MultiplyTool {
    a: f64,
    b: f64,
}

impl MultiplyTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok(format!("{}", self.a * self.b))
    }
}

#[derive(McpTool, Clone, Default)]
#[tool(name = "greet", description = "Greet someone by name")]
struct GreetTool {
    name: String,
}

impl GreetTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok(format!("Hello, {}!", self.name))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("=== Dynamic Tools Server ===");
    info!("Endpoint: http://127.0.0.1:8484/mcp");

    let server = McpServer::builder()
        .name("dynamic-tools-demo")
        .version("0.1.0")
        .tool_change_mode(ToolChangeMode::DynamicInProcess)
        .tool(AddTool::default())
        .tool(MultiplyTool::default())
        .tool(GreetTool::default())
        .bind_address("127.0.0.1:8484".parse()?)
        .build()?;

    let registry = server
        .tool_registry()
        .expect("DynamicInProcess must have registry")
        .clone();

    info!("tools.listChanged = true");
    info!("Tools: add, multiply, greet");
    info!("");
    info!("Connect and open an SSE stream (see source for curl commands).");
    info!("In 15s: 'multiply' deactivated. In 30s: 'multiply' reactivated.");

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(15)).await;
        info!(">>> Deactivating 'multiply' <<<");
        match registry.deactivate_tool("multiply").await {
            Ok(true) => info!("'multiply' deactivated — notification sent"),
            Ok(false) => info!("'multiply' was already inactive"),
            Err(e) => info!("Failed: {}", e),
        }

        tokio::time::sleep(std::time::Duration::from_secs(15)).await;
        info!(">>> Activating 'multiply' <<<");
        match registry.activate_tool("multiply").await {
            Ok(true) => info!("'multiply' activated — notification sent"),
            Ok(false) => info!("'multiply' was already active"),
            Err(e) => info!("Failed: {}", e),
        }
    });

    server.run().await?;
    Ok(())
}
