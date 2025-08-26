//! # Zero-Configuration MCP Server
//!
//! This example demonstrates the **zero-configuration framework** where:
//! - Users NEVER specify method strings
//! - Framework auto-determines ALL methods from types
//! - Simple derive macros replace complex trait implementations
//! - Pluggable session storage (InMemory → SQLite → PostgreSQL → AWS)

use mcp_derive::{McpTool, McpNotification};
use mcp_server::McpServer;

/// ✅ ZERO CONFIGURATION - Framework auto-determines name: "calculator"
#[derive(McpTool)]
struct Calculator {
    #[param(description = "First number")]
    a: f64,
    #[param(description = "Second number")]
    b: f64,
}

impl Calculator {
    async fn execute(&self) -> mcp_server::McpResult<f64> {
        Ok(self.a + self.b)
    }
}

/// ✅ ZERO CONFIGURATION - Framework auto-determines method: "notifications/progress"
#[derive(McpNotification, Default)]
struct ProgressNotification {
    message: String,
    percent: u32,
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("🚀 Starting Zero-Configuration MCP Server");
    println!("   • Framework auto-determines tool name: Calculator → 'calculator'");
    println!("   • Framework auto-determines notification method: ProgressNotification → 'notifications/progress'");
    println!("   • Session storage: InMemorySessionStorage (zero-config default)");
    println!("   • HTTP transport: http://127.0.0.1:8000/mcp");
    println!();

    // ✅ ZERO CONFIGURATION - Framework handles everything automatically
    let server = McpServer::builder()
        .name("zero-config-demo")
        .version("1.0.0")
        .tool(Calculator { a: 0.0, b: 0.0 })                      // Framework → tools/call
        .notification_type::<ProgressNotification>()              // Framework → notifications/progress
        .build()?;

    // Demonstrate notification (would be sent by the framework)
    println!("📈 Progress: Server initialization complete (100%)");

    println!("✅ Server ready! Test with:");
    println!("   curl -X POST http://127.0.0.1:8000/mcp \\");
    println!("     -H 'Content-Type: application/json' \\");
    println!("     -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{{\"name\":\"calculator\",\"arguments\":{{\"a\":5,\"b\":3}}}}}}'");
    println!();

    Ok(server.run().await?)
}