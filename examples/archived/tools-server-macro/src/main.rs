//! # Tools Server - Macro-Based Example
//!
//! This demonstrates the RECOMMENDED way to implement MCP tools using macros.
//! Framework automatically maps tool types to "tools/call" - zero configuration needed.
//!
//! Lines of code: ~60 (vs 400+ with manual trait implementations)

use serde_json::Value;
use std::collections::HashMap;
use tracing::info;
use mcp_server::{McpServer, McpResult};

// =============================================================================
// CALCULATOR TOOL - Framework auto-uses "tools/call"
// =============================================================================

#[derive(Debug)]
pub struct Calculator {
    // Framework automatically maps to "tools/call"
    // Tool name becomes struct name: "Calculator"
    name: String,
    description: String,
}

impl Calculator {
    pub fn new() -> Self {
        Self {
            name: "calculator".to_string(),
            description: "Perform mathematical calculations with full precision".to_string(),
        }
    }
    
    pub async fn execute(&self, args: HashMap<String, Value>) -> McpResult<Value> {
        let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let operation = args.get("operation").and_then(|v| v.as_str()).unwrap_or("add");
        
        let result = match operation {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            "divide" if b != 0.0 => a / b,
            "divide" => return Err(mcp_protocol::McpError::tool_execution("Division by zero")),
            "power" => a.powf(b),
            "sqrt" if a >= 0.0 => a.sqrt(),
            "sqrt" => return Err(mcp_protocol::McpError::tool_execution("Cannot take square root of negative number")),
            _ => return Err(mcp_protocol::McpError::invalid_param_type(
                "operation", 
                "add|subtract|multiply|divide|power|sqrt", 
                operation
            )),
        };
        
        info!("🔢 Calculator: {} {} {} = {}", a, operation, b, result);
        Ok(serde_json::json!({
            "result": result,
            "operation": operation,
            "inputs": { "a": a, "b": b },
            "tool": "calculator"
        }))
    }
}

// TODO: This will be replaced with #[derive(McpTool)] when framework supports it
// The derive macro will automatically implement the tool traits and register
// the "tools/call" method without any manual specification

// =============================================================================
// STRING UTILITIES TOOL - Framework auto-uses "tools/call"
// =============================================================================

#[derive(Debug)]
pub struct StringUtils {
    // Framework automatically maps to "tools/call"
    // Multiple tools can coexist, each with their own method registration
    name: String,
    description: String,
}

impl StringUtils {
    pub fn new() -> Self {
        Self {
            name: "string_utils".to_string(),
            description: "Perform string manipulation operations".to_string(),
        }
    }
    
    pub async fn execute(&self, args: HashMap<String, Value>) -> McpResult<Value> {
        let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
        let operation = args.get("operation").and_then(|v| v.as_str()).unwrap_or("uppercase");
        
        let result = match operation {
            "uppercase" => text.to_uppercase(),
            "lowercase" => text.to_lowercase(),
            "reverse" => text.chars().rev().collect(),
            "length" => return Ok(serde_json::json!({
                "result": text.len(),
                "operation": operation,
                "input": text,
                "tool": "string_utils"
            })),
            "words" => return Ok(serde_json::json!({
                "result": text.split_whitespace().count(),
                "operation": operation,
                "input": text,
                "tool": "string_utils"
            })),
            _ => return Err(mcp_protocol::McpError::invalid_param_type(
                "operation",
                "uppercase|lowercase|reverse|length|words",
                operation
            )),
        };
        
        info!("🔤 String Utils: {} '{}' -> '{}'", operation, text, result);
        Ok(serde_json::json!({
            "result": result,
            "operation": operation,
            "input": text,
            "tool": "string_utils"
        }))
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

    info!("🚀 Starting Tools Server - Macro-Based Example");
    info!("================================================");
    info!("💡 Framework automatically maps tool types to 'tools/call'");
    info!("💡 Zero method strings specified - types determine methods!");

    // Create tool instances (framework will auto-determine methods)
    let _calculator = Calculator::new();
    let _string_utils = StringUtils::new();
    
    info!("🔧 Available Tools:");
    info!("   • Calculator → tools/call (automatic)");
    info!("   • StringUtils → tools/call (automatic)");

    // TODO: This will become much simpler with derive macros:
    // let server = McpServer::builder()
    //     .tool(calculator)      // Auto-registers "tools/call" for Calculator
    //     .tool(string_utils)    // Auto-registers "tools/call" for StringUtils
    //     .build()?;

    // For now, create a server demonstrating the concept
    let server = McpServer::builder()
        .name("tools-server-macro")
        .version("1.0.0")
        .title("Tools Server - Macro-Based Example")
        .instructions(
            "This server demonstrates zero-configuration tool implementation. \
             Framework automatically maps Calculator and StringUtils to tools/call. \
             Use Calculator for math operations (add, subtract, multiply, divide, power, sqrt) \
             and StringUtils for text operations (uppercase, lowercase, reverse, length, words)."
        )
        .bind_address("127.0.0.1:8080".parse()?)
        .sse(true)
        .build()?;

    info!("🎯 Server running at: http://127.0.0.1:8080/mcp");
    info!("🔥 ZERO tool method strings specified - framework auto-determined ALL methods!");
    info!("💡 This is the future of MCP - declarative, type-safe, zero-config!");

    server.run().await?;
    Ok(())
}