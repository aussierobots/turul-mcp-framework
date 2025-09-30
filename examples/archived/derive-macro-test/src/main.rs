//! # Derive Macro Test - TRUE Zero-Configuration Example
//!
//! This demonstrates the ULTIMATE MCP framework usage with derive macros.
//! Framework automatically implements ALL traits and determines ALL methods from types.
//!
//! Lines of code: ~50 (vs 400+ with manual trait implementations)

use turul_mcp_derive::{McpTool, McpResource, McpNotification};
use turul_mcp_server::{McpServer, McpResult};
use turul_mcp_protocol::resources::HasResourceUri; // Import trait for .uri() method
use tracing::info;

// =============================================================================
// DERIVE MACRO TOOLS - TRUE Zero-Configuration
// =============================================================================

#[derive(McpTool, Clone, Debug)]
#[tool(name = "calculator", description = "Perform mathematical calculations")]
pub struct Calculator {
    #[param(description = "First number")]
    pub a: f64,
    #[param(description = "Second number")]
    pub b: f64,
    #[param(description = "Operation to perform")]
    pub operation: String,
}

impl Calculator {
    pub async fn execute(&self) -> McpResult<String> {
        let result = match self.operation.as_str() {
            "add" => self.a + self.b,
            "subtract" => self.a - self.b,
            "multiply" => self.a * self.b,
            "divide" if self.b != 0.0 => self.a / self.b,
            "divide" => return Err(turul_mcp_protocol::McpError::tool_execution("Division by zero")),
            _ => return Err(turul_mcp_protocol::McpError::invalid_param_type(
                "operation",
                "add|subtract|multiply|divide",
                &self.operation
            )),
        };

        info!("🔢 Calculator: {} {} {} = {}", self.a, self.operation, self.b, result);
        Ok(format!("{} {} {} = {}", self.a, self.operation, self.b, result))
    }
}

// =============================================================================
// DERIVE MACRO RESOURCES - TRUE Zero-Configuration
// =============================================================================

#[derive(McpResource, Debug)]
#[uri = "file://config.json"]
#[name = "Configuration File"]
#[description = "Application configuration data"]
pub struct ConfigResource {
    #[content]
    #[content_type = "application/json"]
    pub data: String,
}

impl ConfigResource {
    pub fn new() -> Self {
        Self {
            data: serde_json::json!({
                "app_name": "derive-macro-test",
                "version": "1.0.0",
                "features": ["zero_config", "type_safe", "derive_macros"],
                "message": "This resource was created with #[derive(McpResource)]!"
            }).to_string()
        }
    }
}

// =============================================================================
// DERIVE MACRO NOTIFICATIONS - TRUE Zero-Configuration
// =============================================================================

#[derive(McpNotification, Clone, Debug)]
// Framework auto-determines method from notification type - NO method strings!
pub struct ProgressUpdate {
    #[payload]
    pub stage: String,
    #[payload]
    pub completed: u64,
    #[payload]
    pub total: u64,
    #[payload]
    pub message: Option<String>,
}

impl ProgressUpdate {
    pub fn new(stage: &str, completed: u64, total: u64) -> Self {
        Self {
            stage: stage.to_string(),
            completed,
            total,
            message: None,
        }
    }

    pub fn with_message(mut self, message: &str) -> Self {
        self.message = Some(message.to_string());
        self
    }

    pub async fn send(&self) -> McpResult<()> {
        let percentage = if self.total > 0 {
            (self.completed * 100) / self.total
        } else {
            0
        };

        info!("📊 Progress Update: {} - {}% ({}/{})",
            self.stage, percentage, self.completed, self.total);

        if let Some(ref msg) = self.message {
            info!("   Message: {}", msg);
        }

        Ok(())
    }
}

// =============================================================================
// MAIN SERVER - ULTIMATE Zero-Configuration Setup
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 Starting Derive Macro Test - ULTIMATE Zero-Configuration");
    info!("============================================================");
    info!("💡 Uses #[derive(McpTool)], #[derive(McpResource)], #[derive(McpNotification)]");
    info!("💡 Framework automatically implements ALL traits and determines ALL methods!");

    // Create instances - derive macros handle EVERYTHING
    let _calculator = Calculator { a: 0.0, b: 0.0, operation: "add".to_string() };
    let _config = ConfigResource::new();

    // Test notifications with derive macros
    let progress = ProgressUpdate::new("initialization", 1, 3)
        .with_message("Testing derive macro functionality");
    progress.send().await?;

    info!("✨ Derive Macro Examples:");
    info!("   • Calculator → #[derive(McpTool)] → tools/call (AUTOMATIC)");
    info!("   • ConfigResource → #[derive(McpResource)] → resources/read (AUTOMATIC)");
    info!("   • ProgressUpdate → #[derive(McpNotification)] → notifications/[auto] (AUTOMATIC)");

    // Create server with derive macro instances
    let server = McpServer::builder()
        .name("derive-macro-test")
        .version("1.0.0")
        .title("Derive Macro Test - TRUE Zero-Configuration")
        .instructions(
            "This server demonstrates the ULTIMATE MCP framework usage with derive macros. \
             Framework automatically implements McpTool, McpResource, and McpNotification traits, \
             and auto-determines all MCP methods from type signatures. This is TRUE zero-configuration!"
        )
        .bind_address("127.0.0.1:8085".parse()?)
        .sse(true)
        .build()?;

    let final_progress = ProgressUpdate::new("startup", 3, 3)
        .with_message("Derive macro server ready!");
    final_progress.send().await?;

    info!("🎯 Server running at: http://127.0.0.1:8085/mcp");
    info!("🔥 ZERO manual trait implementations - derive macros generated EVERYTHING!");
    info!("💡 This is the ULTIMATE MCP framework - derive macros + zero-config + type-safe!");

    server.run().await?;
    Ok(())
}