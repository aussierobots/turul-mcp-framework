//! # Zero-Configuration MCP Server
//!
//! This example demonstrates the **zero-configuration framework** where:
//! - Users NEVER specify method strings
//! - Framework auto-determines ALL methods from types
//! - Simple derive macros replace complex trait implementations
//! - Pluggable session storage (InMemory → SQLite → PostgreSQL → AWS)

use turul_mcp_derive::McpTool;
use turul_mcp_server::McpServer;

/// ZERO CONFIGURATION - Framework auto-determines name: "calculator"
#[derive(McpTool)]
struct Calculator {
    #[param(description = "First number")]
    a: f64,
    #[param(description = "Second number")]
    b: f64,
}

impl Calculator {
    async fn execute(
        &self,
        _session: Option<turul_mcp_server::SessionContext>,
    ) -> turul_mcp_server::McpResult<f64> {
        Ok(self.a + self.b)
    }
}


/// ZERO CONFIGURATION - Framework auto-determines name: "log_progress"
/// This tool demonstrates SessionContext usage for real-time notifications
#[derive(McpTool)]
struct LogProgress {
    #[param(description = "Progress message")]
    message: String,
    #[param(description = "Completion percentage (0-100)")]
    percent: u32,
}

impl LogProgress {
    async fn execute(
        &self,
        session: Option<turul_mcp_server::SessionContext>,
    ) -> turul_mcp_server::McpResult<String> {
        if let Some(session) = session {
            // Send built-in MCP progress notification to connected clients via SSE
            session.notify_progress(&self.message, self.percent as u64).await;
            tracing::info!("Sent MCP progress notification: {} ({}%)", self.message, self.percent);
        }
        
        Ok(format!("Progress logged: {} ({}% complete)", self.message, self.percent))
    }
}

/// Test struct return type  
#[derive(serde::Serialize)]
struct CalculationResult {
    sum: f64,
    operation: String,
    timestamp: u64,
}


/// ZERO CONFIGURATION - Framework auto-determines name: "struct_calculator"
#[derive(McpTool)]
struct StructCalculator {
    #[param(description = "First number")]
    a: f64,
    #[param(description = "Second number")]
    b: f64,
}

impl StructCalculator {
    async fn execute(
        &self,
        _session: Option<turul_mcp_server::SessionContext>,
    ) -> turul_mcp_server::McpResult<CalculationResult> {
        Ok(CalculationResult {
            sum: self.a + self.b,
            operation: "addition".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }
}

/// Enhanced calculator demonstrating custom field name for primitive output
#[derive(McpTool)]
#[tool(name = "enhanced_calculator", description = "Enhanced calculator with custom field name", field = "result")]
struct EnhancedCalculator {
    #[param(description = "First number")]
    x: f64,
    #[param(description = "Second number")]
    y: f64,
}

impl EnhancedCalculator {
    async fn execute(
        &self,
        _session: Option<turul_mcp_server::SessionContext>,
    ) -> turul_mcp_server::McpResult<f64> {
        Ok(self.x * self.y)  // Simple multiplication 
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("Starting Zero-Configuration MCP Server");
    println!("   Framework auto-determines tool name: Calculator → 'calculator'");
    println!("   Framework auto-determines tool name: LogProgress → 'log_progress'");
    println!("   Framework auto-determines tool name: StructCalculator → 'struct_calculator'");
    println!("   Explicit tool definition: EnhancedCalculator → 'enhanced_calculator' (with custom field = 'result')");
    println!("   Built-in MCP notifications: progress, logging, resources (no registration needed)");
    println!("   Session storage: InMemorySessionStorage (zero-config default)");
    println!("   HTTP transport: http://127.0.0.1:8641/mcp");
    println!();

    // ZERO CONFIGURATION - Framework handles everything automatically
    let server = McpServer::builder()
        .name("zero-config-demo")
        .version("1.0.0")
        .tool(Calculator { a: 0.0, b: 0.0 }) // Framework → tools/call  
        .tool(LogProgress { message: String::new(), percent: 0 }) // Framework → tools/call
        .tool(StructCalculator { a: 0.0, b: 0.0 }) // Framework → tools/call
        .tool(EnhancedCalculator { x: 0.0, y: 0.0 }) // Explicit output type + custom field → tools/call
        .bind_address("127.0.0.1:8641".parse()?)
        .sse(true) // Enable SSE for real-time MCP notifications
        .build()?;

    println!("✅ Server configured with zero manual configuration!");

    println!("Server ready! Test with:");
    println!("1. Calculator tool:");
    println!("   curl -X POST http://127.0.0.1:8641/mcp \\");
    println!("     -H 'Content-Type: application/json' \\");
    println!(
        "     -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{{\"name\":\"calculator\",\"arguments\":{{\"a\":5,\"b\":3}}}}}}'"
    );
    println!();
    println!("2. Progress logging tool (sends real-time MCP notifications):");
    println!("   curl -X POST http://127.0.0.1:8641/mcp \\");
    println!("     -H 'Content-Type: application/json' \\");
    println!(
        "     -d '{{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{{\"name\":\"log_progress\",\"arguments\":{{\"message\":\"Processing data\",\"percent\":75}}}}}}'"
    );
    println!();
    println!("3. Monitor MCP progress notifications (SSE):");
    println!("   curl -N -H 'Accept: text/event-stream' http://127.0.0.1:8641/mcp");
    println!();

    Ok(server.run().await?)
}
