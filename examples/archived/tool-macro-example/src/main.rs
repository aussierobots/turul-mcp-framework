//! # Tool Macro Example
//!
//! This example demonstrates the `tool!` declarative macro for creating simple MCP tools
//! with inline parameter definitions and execute closures.

use mcp_derive::tool;
use mcp_server::{McpServer, McpTool};
use mcp_protocol::tools::{HasBaseMetadata, HasDescription, HasInputSchema};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🎯 Testing tool! declarative macro");
    println!("✨ Demonstrating inline tool creation with constraints");

    // Create tools using the tool! declarative macro
    let divide_tool = tool! {
        name: "divide",
        description: "Divide two numbers with validation",
        params: {
            a: f64 => "Dividend (first number)",
            b: f64 => "Divisor (second number)",
        },
        execute: |a: f64, b: f64| async move {
            if b == 0.0 {
                Err("Division by zero")
            } else {
                Ok(format!("{} ÷ {} = {}", a, b, a / b))
            }
        }
    };

    let add_tool = tool! {
        name: "add",
        description: "Add two numbers together",
        params: {
            x: f64 => "First number to add",
            y: f64 => "Second number to add",
        },
        execute: |x: f64, y: f64| async move {
            Ok::<String, String>(format!("{} + {} = {}", x, y, x + y))
        }
    };

    let greet_tool = tool! {
        name: "greet",
        description: "Greet someone by name with style options",
        params: {
            name: String => "Name of person to greet",
            formal: bool => "Use formal greeting style",
        },
        execute: |name: String, formal: bool| async move {
            let greeting = if formal {
                format!("Good day, {}!", name)
            } else {
                format!("Hey there, {}!", name)
            };
            Ok::<String, String>(greeting)
        }
    };

    // Text processing tool with validation
    let text_tool = tool! {
        name: "text_process",
        description: "Process text with various operations",
        params: {
            text: String => "Text to process",
            operation: String => "Operation (uppercase, lowercase, reverse)",
        },
        execute: |text: String, operation: String| async move {
            match operation.as_str() {
                "uppercase" => Ok(text.to_uppercase()),
                "lowercase" => Ok(text.to_lowercase()),
                "reverse" => Ok(text.chars().rev().collect()),
                _ => Err(format!("Unknown operation: {}. Use: uppercase, lowercase, reverse", operation)),
            }
        }
    };

    // Test the tools
    println!("\n📋 Testing tools created with tool! macro:");
    
    println!("\n1. Testing divide tool:");
    println!("   Name: {}", divide_tool.name());
    println!("   Description: {}", divide_tool.description().unwrap_or("No description"));
    
    let test_args = serde_json::json!({ "a": 10.0, "b": 2.0 });
    match divide_tool.call(test_args, None).await {
        Ok(results) => println!("   ✅ Result: {:?}", results),
        Err(e) => println!("   ❌ Error: {}", e),
    }

    println!("\n2. Testing add tool:");
    println!("   Name: {}", add_tool.name());
    println!("   Description: {}", add_tool.description().unwrap_or("No description"));
    
    let test_args = serde_json::json!({ "x": 5.5, "y": 3.2 });
    match add_tool.call(test_args, None).await {
        Ok(results) => println!("   ✅ Result: {:?}", results),
        Err(e) => println!("   ❌ Error: {}", e),
    }

    println!("\n3. Testing greet tool:");
    println!("   Name: {}", greet_tool.name());
    println!("   Description: {}", greet_tool.description().unwrap_or("No description"));
    
    let test_args = serde_json::json!({ "name": "Alice", "formal": false });
    match greet_tool.call(test_args, None).await {
        Ok(results) => println!("   ✅ Result: {:?}", results),
        Err(e) => println!("   ❌ Error: {}", e),
    }

    println!("\n4. Testing text processing tool:");
    println!("   Name: {}", text_tool.name());
    println!("   Description: {}", text_tool.description().unwrap_or("No description"));
    
    let test_args = serde_json::json!({ "text": "Hello World", "operation": "reverse" });
    match text_tool.call(test_args, None).await {
        Ok(results) => println!("   ✅ Result: {:?}", results),
        Err(e) => println!("   ❌ Error: {}", e),
    }

    // Test error cases
    println!("\n🔍 Testing error handling:");
    
    println!("\n   Division by zero:");
    let test_args = serde_json::json!({ "a": 10.0, "b": 0.0 });
    match divide_tool.call(test_args, None).await {
        Ok(results) => println!("   ⚠️  Unexpected result: {:?}", results),
        Err(e) => println!("   ✅ Expected error: {}", e),
    }

    println!("\n   Invalid text operation:");
    let test_args = serde_json::json!({ "text": "test", "operation": "invalid" });
    match text_tool.call(test_args, None).await {
        Ok(results) => println!("   ⚠️  Unexpected result: {:?}", results),
        Err(e) => println!("   ✅ Expected error: {}", e),
    }

    // Create and run a simple server with these tools
    let server = McpServer::builder()
        .name("tool-macro-example")
        .version("1.0.0")
        .title("Tool Macro Example Server")
        .instructions("This server demonstrates the tool! declarative macro for creating MCP tools with inline parameter definitions and execute closures.")
        .tool(divide_tool)
        .tool(add_tool)
        .tool(greet_tool)
        .tool(text_tool)
        .bind_address("127.0.0.1:8010".parse()?)
        .build()?;

    println!("\n🚀 Starting server at: http://127.0.0.1:8010/mcp");
    println!("📋 Tools available:");
    println!("  📊 divide: Divide two numbers with validation");
    println!("  ➕ add: Add two numbers together");
    println!("  👋 greet: Greet someone by name with style options");
    println!("  📝 text_process: Process text with various operations");
    println!("\n✅ Declarative macro features demonstrated:");
    println!("  ✨ Inline tool creation with tool! macro");
    println!("  ✨ Parameter definitions with type and description");
    println!("  ✨ Closure-based execution logic");
    println!("  ✨ Automatic schema generation");
    println!("  ✨ Error handling and validation");

    server.run().await?;
    Ok(())
}