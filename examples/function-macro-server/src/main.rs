//! # Function Macro Server Example
//!
//! This example demonstrates the #[mcp_tool] function attribute macro for creating
//! MCP tools from regular async functions with parameter attributes.

use turul_mcp_derive::mcp_tool;
use turul_mcp_server::{McpServer, McpResult};
use turul_mcp_protocol::McpError;

// Test basic function macro - addition tool
#[mcp_tool(name = "add", description = "Add two numbers together")]
async fn add_numbers(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<String> {
    Ok(format!("{} + {} = {}", a, b, a + b))
}

// String repeat tool - demonstrates different parameter types  
#[mcp_tool(name = "string_repeat", description = "Repeat a string multiple times")]
async fn repeat_string(
    #[param(description = "Text to repeat")] text: String,
    #[param(description = "Number of repetitions")] count: i32,
) -> McpResult<String> {
    if count < 0 {
        return Err(McpError::param_out_of_range("count", &count.to_string(), "must be non-negative"));
    }
    if count > 1000 {
        return Err(McpError::param_out_of_range("count", &count.to_string(), "maximum 1000"));
    }
    
    Ok(text.repeat(count as usize))
}

// Boolean logic tool - demonstrates string enum and boolean parameters
#[mcp_tool(name = "boolean_logic", description = "Perform boolean operations")]
async fn boolean_logic(
    #[param(description = "First boolean value")] a: bool,
    #[param(description = "Second boolean value")] b: bool, 
    #[param(description = "Boolean operation to perform")] operation: String,
) -> McpResult<String> {
    let result = match operation.as_str() {
        "and" => a && b,
        "or" => a || b,
        "xor" => a ^ b,
        _ => return Err(McpError::invalid_param_type("operation", "and|or|xor", &operation)),
    };
    
    Ok(format!("{} {} {} = {}", a, operation, b, result))
}

// Optional parameter demonstration
#[mcp_tool(name = "greet", description = "Greet someone with optional custom message")]
async fn greet_person(
    #[param(description = "Name of person to greet")] name: String,
    #[param(description = "Optional custom greeting", optional)] greeting: Option<String>,
) -> McpResult<String> {
    let greeting = greeting.unwrap_or_else(|| "Hello".to_string());
    Ok(format!("{}, {}!", greeting, name))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🎯 Starting Function Macro MCP Server");
    println!("✨ Demonstrating #[mcp_tool] function attribute macro");

    let server = McpServer::builder()
        .name("function-macro-server")
        .version("1.0.0")
        .title("Function Macro Example Server")
        .instructions("This server demonstrates the #[mcp_tool] function attribute macro for creating MCP tools from regular async functions with parameter attributes.")
        .tool(AddNumbersToolImpl)
        .tool(RepeatStringToolImpl)
        .tool(BooleanLogicToolImpl)
        .tool(GreetPersonToolImpl)
        .bind_address("127.0.0.1:8003".parse()?)
        .build()?;

    println!("🚀 Function Macro server running at: http://127.0.0.1:8003/mcp");
    println!("📋 Available tools generated from functions:");
    println!("  📊 add: Add two numbers (f64, f64) → String");
    println!("  🔁 string_repeat: Repeat text (String, i32) → String");
    println!("  🔣 boolean_logic: Boolean operations (bool, bool, String) → String");
    println!("  👋 greet: Greet with optional message (String, Option<String>) → String");
    println!("\n✅ Macro syntax successfully implemented:");
    println!("  #[mcp_tool(name = \"add\", description = \"Add two numbers\")]");
    println!("  async fn add_numbers(");
    println!("      #[param(description = \"First number\")] a: f64,");
    println!("      #[param(description = \"Second number\")] b: f64,");
    println!("  ) -> McpResult<String> {{ /* ... */ }}");
    println!("\n🎨 Features demonstrated:");
    println!("  ✨ Automatic McpTool trait implementation");
    println!("  ✨ JSON Schema generation from types"); 
    println!("  ✨ Parameter validation and extraction");
    println!("  ✨ Optional parameter support");
    println!("  ✨ Multiple parameter types (String, f64, i32, bool)");

    server.run().await?;
    Ok(())
}