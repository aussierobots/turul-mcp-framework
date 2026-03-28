//! # Dynamic Tools Server
//!
//! Demonstrates runtime tool activation/deactivation with MCP-compliant
//! `notifications/tools/list_changed` notifications.
//!
//! ## How to test
//!
//! 1. Start the server:
//!    ```
//!    cargo run -p dynamic-tools-server
//!    ```
//!
//! 2. Connect with MCP Inspector at http://127.0.0.1:8484/mcp
//!
//! 3. After initializing, call `tools/list` — you should see 3 tools:
//!    `add`, `multiply`, `greet`
//!
//! 4. Wait 10 seconds — the server auto-deactivates `multiply`
//!    → If you have an SSE stream open, you receive `notifications/tools/list_changed`
//!    → Call `tools/list` again — `multiply` is gone
//!    → Call `tools/call` for `multiply` — you get ToolNotFound error
//!
//! 5. Wait 10 more seconds — the server auto-activates `multiply`
//!    → Another `notifications/tools/list_changed` notification
//!    → `multiply` reappears in `tools/list`

use tracing::info;
use turul_mcp_derive::McpTool;
use turul_mcp_server::{McpResult, McpServer, SessionContext, ToolChangeMode};

// --- Tools that can be activated/deactivated ---

#[derive(McpTool, Clone, Default)]
#[tool(name = "add", description = "Add two numbers")]
struct AddTool {
    a: f64,
    b: f64,
}

impl AddTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<f64> {
        Ok(self.a + self.b)
    }
}

#[derive(McpTool, Clone, Default)]
#[tool(name = "multiply", description = "Multiply two numbers")]
struct MultiplyTool {
    a: f64,
    b: f64,
}

impl MultiplyTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<f64> {
        Ok(self.a * self.b)
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

    info!("Starting Dynamic Tools Server on http://127.0.0.1:8484/mcp");

    let server = McpServer::builder()
        .name("dynamic-tools-demo")
        .version("0.1.0")
        .tool_change_mode(ToolChangeMode::DynamicInProcess)
        .tool(AddTool::default())
        .tool(MultiplyTool::default())
        .tool(GreetTool::default())
        .bind_address("127.0.0.1:8484".parse()?)
        .build()?;

    info!("tools.listChanged = true");
    info!("Connect with MCP Inspector, then watch for tool changes...");
    info!("  - In 10s: 'multiply' will be deactivated");
    info!("  - In 20s: 'multiply' will be reactivated");

    // Get the tool registry for the demo task
    let registry = server
        .tool_registry()
        .expect("DynamicInProcess mode must have a registry")
        .clone();

    // Auto-demo: deactivate after 10s, reactivate after 20s
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        info!("--- Deactivating 'multiply' ---");
        match registry.deactivate_tool("multiply").await {
            Ok(true) => info!("'multiply' deactivated — notification sent to connected clients"),
            Ok(false) => info!("'multiply' was already inactive"),
            Err(e) => info!("Failed: {}", e),
        }

        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        info!("--- Activating 'multiply' ---");
        match registry.activate_tool("multiply").await {
            Ok(true) => info!("'multiply' activated — notification sent to connected clients"),
            Ok(false) => info!("'multiply' was already active"),
            Err(e) => info!("Failed: {}", e),
        }
    });

    server.run().await?;
    Ok(())
}
