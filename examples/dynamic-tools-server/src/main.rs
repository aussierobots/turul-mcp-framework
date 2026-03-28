//! # Dynamic Tools Server
//!
//! Demonstrates runtime tool activation/deactivation with MCP-compliant
//! `notifications/tools/list_changed` notifications.
//!
//! ## Usage
//!
//! Start with multiply active (default):
//!   cargo run -p dynamic-tools-server
//!
//! Start with multiply inactive:
//!   cargo run -p dynamic-tools-server -- --multiply-inactive
//!
//! ## Testing with MCP Inspector
//!
//! 1. Connect at http://127.0.0.1:8484/mcp
//! 2. Call tools/list — see add, multiply, greet, activate_multiply OR deactivate_multiply
//! 3. Call deactivate_multiply — triggers notifications/tools/list_changed
//! 4. Call tools/list — multiply is gone, deactivate_multiply replaced by activate_multiply
//! 5. Call activate_multiply — multiply reappears

use std::sync::{Arc, OnceLock};

use tracing::info;
use turul_mcp_derive::McpTool;
use turul_mcp_protocol::tools::{CallToolResult, ToolResult};
use turul_mcp_server::{McpResult, McpServer, SessionContext, ToolChangeMode, ToolRegistry};

/// Global registry handle so toggle tools can access it from execute()
static REGISTRY: OnceLock<Arc<ToolRegistry>> = OnceLock::new();

// --- Math tools ---

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

// --- Toggle tools (only one is active at a time) ---

#[derive(McpTool, Clone, Default)]
#[tool(
    name = "activate_multiply",
    description = "Activate the multiply tool. Triggers notifications/tools/list_changed."
)]
struct ActivateMultiplyTool {}

impl ActivateMultiplyTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<CallToolResult> {
        let registry = REGISTRY.get().expect("Registry not initialized");
        registry.activate_tool("multiply").await.map_err(|e| {
            turul_mcp_protocol::McpError::ToolExecutionError(e.to_string())
        })?;
        // Swap: hide activate, show deactivate
        let _ = registry.deactivate_tool("activate_multiply").await;
        let _ = registry.activate_tool("deactivate_multiply").await;
        Ok(CallToolResult::success(vec![ToolResult::text(
            "multiply activated. Call tools/list to see updated tools.",
        )]))
    }
}

#[derive(McpTool, Clone, Default)]
#[tool(
    name = "deactivate_multiply",
    description = "Deactivate the multiply tool. Triggers notifications/tools/list_changed."
)]
struct DeactivateMultiplyTool {}

impl DeactivateMultiplyTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<CallToolResult> {
        let registry = REGISTRY.get().expect("Registry not initialized");
        registry.deactivate_tool("multiply").await.map_err(|e| {
            turul_mcp_protocol::McpError::ToolExecutionError(e.to_string())
        })?;
        // Swap: hide deactivate, show activate
        let _ = registry.deactivate_tool("deactivate_multiply").await;
        let _ = registry.activate_tool("activate_multiply").await;
        Ok(CallToolResult::success(vec![ToolResult::text(
            "multiply deactivated. Call tools/list to see updated tools.",
        )]))
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

    let multiply_active = !std::env::args().any(|a| a == "--multiply-inactive");

    info!("=== Dynamic Tools Server ===");
    info!("Endpoint: http://127.0.0.1:8484/mcp");
    info!("multiply starts {}", if multiply_active { "active" } else { "inactive" });

    let server = McpServer::builder()
        .name("dynamic-tools-demo")
        .version("0.1.0")
        .tool_change_mode(ToolChangeMode::DynamicInProcess)
        .tool(AddTool::default())
        .tool(MultiplyTool::default())
        .tool(GreetTool::default())
        .tool(ActivateMultiplyTool::default())
        .tool(DeactivateMultiplyTool::default())
        .bind_address("127.0.0.1:8484".parse()?)
        .build()?;

    let registry = server
        .tool_registry()
        .expect("DynamicInProcess must have registry")
        .clone();

    // Store registry globally so toggle tools can access it
    if REGISTRY.set(registry.clone()).is_err() {
        panic!("Registry already set");
    }

    // Set initial state based on flag
    if multiply_active {
        // multiply active → hide activate_multiply, show deactivate_multiply
        let _ = registry.deactivate_tool("activate_multiply").await;
    } else {
        // multiply inactive → hide multiply and deactivate_multiply, show activate_multiply
        let _ = registry.deactivate_tool("multiply").await;
        let _ = registry.deactivate_tool("deactivate_multiply").await;
    }

    let active_tools = registry.list_active_tools().await;
    let tool_names: Vec<&str> = active_tools.iter().map(|t| t.name.as_str()).collect();
    info!("Active tools: {:?}", tool_names);
    info!("tools.listChanged = true");

    server.run().await?;
    Ok(())
}
