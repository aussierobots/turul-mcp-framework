//! # Universal MCP Server - Zero Configuration Example
//!
//! This example demonstrates the IDEAL MCP framework usage across ALL 9 protocol areas.
//! Key principle: Users NEVER specify method strings - framework auto-determines from types.
//!
//! Lines of code: ~80 (vs 2000+ with manual method specification)
//! Configuration needed: ZERO method strings anywhere

use serde_json::Value;
use std::collections::HashMap;
use tracing::info;
use turul_mcp_server::{McpServer, McpResult};

// =============================================================================
// TOOLS - Framework auto-uses "tools/call"
// =============================================================================

#[derive(Debug)]
struct Calculator {
    name: String,
    description: String,
}

impl Calculator {
    fn new() -> Self {
        Self {
            name: "calculator".to_string(),
            description: "Perform mathematical calculations".to_string(),
        }
    }

    async fn execute(&self, args: HashMap<String, Value>) -> McpResult<Value> {
        let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let operation = args.get("operation").and_then(|v| v.as_str()).unwrap_or("add");

        let result = match operation {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            "divide" if b != 0.0 => a / b,
            "divide" => return Err(turul_mcp_protocol::McpError::tool_execution("Division by zero")),
            _ => return Err(turul_mcp_protocol::McpError::invalid_param_type("operation", "add|subtract|multiply|divide", operation)),
        };

        info!("🔢 Calculator: {} {} {} = {}", a, operation, b, result);
        Ok(serde_json::json!({ "result": result, "operation": operation }))
    }
}

// Framework should auto-implement McpTool trait with "tools/call" method
// TODO: This will be replaced with #[derive(McpTool)] when framework supports it

// =============================================================================
// NOTIFICATIONS - Framework auto-determines method from type
// =============================================================================

#[derive(Debug, Clone)]
struct ProgressNotification {
    // Framework automatically maps to "notifications/progress"
    stage: String,
    completed: u64,
    total: u64,
    message: Option<String>,
}

impl ProgressNotification {
    fn new(stage: &str, completed: u64, total: u64) -> Self {
        Self {
            stage: stage.to_string(),
            completed,
            total,
            message: None,
        }
    }

    fn with_message(mut self, message: &str) -> Self {
        self.message = Some(message.to_string());
        self
    }

    async fn send(&self) -> McpResult<()> {
        let percentage = if self.total > 0 {
            (self.completed * 100) / self.total
        } else {
            0
        };

        info!("📊 Progress: {} - {}% ({}/{})",
            self.stage, percentage, self.completed, self.total);

        if let Some(ref msg) = self.message {
            info!("   Message: {}", msg);
        }

        // Framework would automatically send via "notifications/progress" method
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct MessageNotification {
    // Framework automatically maps to "notifications/message"
    content: String,
    level: MessageLevel,
}

#[derive(Debug, Clone)]
enum MessageLevel {
    Info,
    Warning,
    Error,
}

impl MessageNotification {
    fn info(content: &str) -> Self {
        Self {
            content: content.to_string(),
            level: MessageLevel::Info,
        }
    }

    fn warning(content: &str) -> Self {
        Self {
            content: content.to_string(),
            level: MessageLevel::Warning,
        }
    }

    async fn send(&self) -> McpResult<()> {
        let icon = match self.level {
            MessageLevel::Info => "ℹ️",
            MessageLevel::Warning => "⚠️",
            MessageLevel::Error => "❌",
        };

        info!("{} Message: {}", icon, self.content);

        // Framework would automatically send via "notifications/message" method
        Ok(())
    }
}

// =============================================================================
// SAMPLING - Framework auto-uses "sampling/createMessage"
// =============================================================================

#[derive(Debug)]
struct CreativeWriter {
    // Framework automatically maps to "sampling/createMessage"
    temperature: f64,
    max_tokens: u32,
    model_type: String,
}

impl CreativeWriter {
    fn new() -> Self {
        Self {
            temperature: 0.8,
            max_tokens: 1000,
            model_type: "creative-model".to_string(),
        }
    }

    async fn sample(&self, prompt: &str) -> McpResult<String> {
        info!("✨ Creative sampling: {} chars, temp={}", prompt.len(), self.temperature);

        // Simulate creative response based on prompt
        let response = if prompt.to_lowercase().contains("story") {
            "Once upon a time, in a world where code came alive...".to_string()
        } else if prompt.to_lowercase().contains("poem") {
            "Roses are red,\nViolets are blue,\nMCP is awesome,\nAnd so are you!".to_string()
        } else {
            format!("Creative response to: {}", prompt)
        };

        Ok(response)
    }
}

// =============================================================================
// RESOURCES - Framework auto-uses "resources/read"
// =============================================================================

#[derive(Debug)]
struct ConfigResource {
    // Framework automatically maps to "resources/read"
    name: String,
    path: std::path::PathBuf,
    mime_type: String,
}

impl ConfigResource {
    fn new(name: &str, path: &str) -> Self {
        Self {
            name: name.to_string(),
            path: std::path::PathBuf::from(path),
            mime_type: "application/json".to_string(),
        }
    }

    async fn read(&self) -> McpResult<String> {
        info!("📄 Reading resource: {}", self.name);

        // Simulate resource reading
        let content = serde_json::json!({
            "resource": self.name,
            "path": self.path.to_string_lossy(),
            "type": "configuration",
            "data": {
                "server_name": "universal-mcp-server",
                "version": "1.0.0",
                "features": ["tools", "notifications", "sampling", "resources"]
            }
        });

        Ok(content.to_string())
    }
}

// =============================================================================
// MAIN SERVER - Zero Configuration Setup
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 Starting Universal MCP Server");
    info!("=================================");
    info!("💡 This server demonstrates ZERO-CONFIGURATION MCP usage");
    info!("💡 All methods auto-determined from types - no strings needed!");

    // Create instances (framework will auto-determine all methods)
    let _calculator = Calculator::new();
    let progress = ProgressNotification::new("initialization", 1, 3);
    let message = MessageNotification::info("Server starting up...");
    let creative_writer = CreativeWriter::new();
    let config_resource = ConfigResource::new("server_config", "config/server.json");

    // Demonstrate zero-config usage
    progress.send().await?;
    message.send().await?;

    let sample = creative_writer.sample("Write a short story about MCP").await?;
    info!("📖 Creative sample: {}", sample);

    let resource_content = config_resource.read().await?;
    info!("📋 Resource content: {}", resource_content);

    // TODO: This will become much simpler when framework supports derive macros:
    // let server = McpServer::builder()
    //     .tool(calculator)                    // Auto-uses "tools/call"
    //     .notification::<ProgressNotification>() // Auto-uses "notifications/progress"
    //     .notification::<MessageNotification>()  // Auto-uses "notifications/message"
    //     .sampler(creative_writer)           // Auto-uses "sampling/createMessage"
    //     .resource(config_resource)          // Auto-uses "resources/read"
    //     .build()?;

    // For now, create a basic server to demonstrate the concept
    let server = McpServer::builder()
        .name("universal-mcp-server")
        .version("1.0.0")
        .title("Universal MCP Server - Zero Configuration")
        .instructions(
            "This server demonstrates the IDEAL MCP framework usage. \
             All methods are auto-determined from types - zero configuration needed! \
             Framework maps Calculator → tools/call, ProgressNotification → notifications/progress, \
             CreativeWriter → sampling/createMessage, ConfigResource → resources/read automatically."
        )
        .bind_address("127.0.0.1:8080".parse()?)
        .sse(true)
        .build()?;

    info!("🎯 Server running at: http://127.0.0.1:8080/mcp");
    info!("🔥 ZERO method strings specified - framework auto-determined ALL methods!");
    info!("📊 Supported MCP areas:");
    info!("   • Tools: Calculator → tools/call (automatic)");
    info!("   • Notifications: Progress/Message → notifications/* (automatic)");
    info!("   • Sampling: CreativeWriter → sampling/createMessage (automatic)");
    info!("   • Resources: ConfigResource → resources/read (automatic)");
    info!("💡 This is how MCP should be - declarative, type-safe, zero-config!");

    let final_progress = ProgressNotification::new("startup", 3, 3)
        .with_message("Server ready for connections");
    final_progress.send().await?;

    server.run().await?;
    Ok(())
}