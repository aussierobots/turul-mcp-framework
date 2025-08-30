//! # Simple Zero-Config Demo - Developer Friendly Patterns  
//!
//! This demonstrates the CURRENT zero-configuration capabilities using
//! the SIMPLE patterns developers actually want to use.
//!
//! Total lines: ~40 (following calculator-add-function-server pattern)

use turul_mcp_derive::mcp_tool;
use turul_mcp_server::{McpServer, McpResult, ToolBuilder};
use serde_json::json;
use tracing::info;

// =============================================================================
// PATTERN 1: Function Macro (Just like calculator-add-function-server)
// =============================================================================

#[mcp_tool(name = "add", description = "Add two numbers")]
async fn add_numbers(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<f64> {
    Ok(a + b)
}

#[mcp_tool(name = "multiply", description = "Multiply two numbers")]
async fn multiply_numbers(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<f64> {
    Ok(a * b)
}

// =============================================================================
// MAIN SERVER - Simple Zero-Config Setup
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 Simple Zero-Config Demo - Developer Friendly!");

    // Pattern 2: Builder Pattern (just like calculator-add-builder-server)
    let divide_tool = ToolBuilder::new("divide")
        .description("Divide two numbers")
        .number_param("a", "Dividend")
        .number_param("b", "Divisor")
        .number_output()
        .execute(|args| async move {
            let a = args.get("a").and_then(|v| v.as_f64()).ok_or("Missing 'a'")?;
            let b = args.get("b").and_then(|v| v.as_f64()).ok_or("Missing 'b'")?;
            if b == 0.0 { return Err("Division by zero".into()); }
            Ok(json!({"result": a / b}))
        })
        .build()?;

    // 🔥 ZERO-CONFIG SERVER SETUP - SIMPLE & WORKS NOW!
    let server = McpServer::builder()
        .name("simple-zero-config-demo")
        .version("1.0.0")
        .title("Simple Zero-Config Demo")
        .instructions("Demonstrating developer-friendly zero-config patterns")
        // Pattern 1: Function macros - uses actual function names!
        .tool_fn(add_numbers)          // ✅ Auto-registers "tools/call" for add
        .tool_fn(multiply_numbers)     // ✅ Auto-registers "tools/call" for multiply  
        // Pattern 2: Builder pattern - runtime flexibility!
        .tool(divide_tool)             // ✅ Auto-registers "tools/call" for divide
        .bind_address("127.0.0.1:8087".parse()?)
        .sse(true)
        .build()?;

    info!("✨ Zero-Config Patterns:");
    info!("   • Function Macros: #[mcp_tool] + .tool_fn(function_name)");
    info!("   • Builder Pattern: ToolBuilder::new().execute().build()");
    info!("   • Server Setup: .tool() and .tool_fn() - framework handles the rest!");
    info!("🎯 Server running at: http://127.0.0.1:8087/mcp");

    server.run().await?;
    Ok(())
}