use async_trait::async_trait;
use mcp_derive::{mcp_tool, tool, McpTool};
use mcp_protocol::{CallToolResponse, ToolResult, ToolSchema};
use mcp_server::{McpResult, McpServer, McpTool, SessionContext};
use serde_json::Value;
use tracing::info;

// APPROACH 1: Derive Macro with Structured Content
#[derive(McpTool, Clone)]
#[tool(name = "approach1_multiply", description = "Multiply two numbers using derive macro")]
#[output_type(f64)]
struct Approach1MultiplyTool {
    #[param(description = "First number")]
    a: f64,
    #[param(description = "Second number")]
    b: f64,
}

impl Approach1MultiplyTool {
    async fn execute(&self) -> McpResult<f64> {
        Ok(self.a * self.b)
    }
}

// APPROACH 2: Function Attribute Macro
#[mcp_tool(name = "approach2_power", description = "Calculate power of a number using function macro")]
async fn approach2_power(
    #[param(description = "Base number")] base: f64,
    #[param(description = "Exponent")] exp: f64,
) -> McpResult<String> {
    let result = base.powf(exp);
    Ok(format!("{} ^ {} = {}", base, exp, result))
}

// APPROACH 3: Manual Trait Implementation with Full Control
#[derive(Clone)]
struct Approach3SqrtTool;

#[async_trait]
impl McpTool for Approach3SqrtTool {
    fn name(&self) -> &str {
        "approach3_sqrt"
    }

    fn description(&self) -> &str {
        "Calculate square root using manual trait implementation"
    }

    fn input_schema(&self) -> ToolSchema {
        use mcp_protocol::schema::JsonSchema;
        use std::collections::HashMap;
        
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("number".to_string(), JsonSchema::number().with_description("Number to calculate square root of")),
            ]))
            .with_required(vec!["number".to_string()])
    }
    
    fn output_schema(&self) -> Option<ToolSchema> {
        use mcp_protocol::schema::JsonSchema;
        use std::collections::HashMap;
        
        Some(ToolSchema::object()
            .with_properties(HashMap::from([
                ("value".to_string(), JsonSchema::number()),
                ("message".to_string(), JsonSchema::string()),
            ]))
            .with_required(vec!["value".to_string(), "message".to_string()]))
    }

    async fn call(&self, args: Value, _session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let number = args.get("number")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| mcp_protocol::McpError::invalid_param_type("number", "number", "other"))?;
        
        if number < 0.0 {
            return Err(mcp_protocol::McpError::tool_execution("Cannot calculate square root of negative number"));
        }
        
        let result = number.sqrt();
        let response = serde_json::json!({
            "value": result,
            "message": format!("√{} = {}", number, result)
        });
        
        Ok(vec![ToolResult::text(response.to_string())])
    }
    
    async fn execute(&self, args: Value, session: Option<SessionContext>) -> McpResult<CallToolResponse> {
        let content = self.call(args.clone(), session).await?;
        let response = CallToolResponse::success(content);
        
        if self.output_schema().is_some() {
            let number = args.get("number").and_then(|v| v.as_f64()).unwrap_or(0.0);
            
            if number >= 0.0 {
                let result = number.sqrt();
                let structured_content = serde_json::json!({
                    "value": result,
                    "message": format!("√{} = {}", number, result)
                });
                Ok(response.with_structured_content(structured_content))
            } else {
                Ok(response) // No structured content for error case
            }
        } else {
            Ok(response)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 Demonstrating all three MCP tool creation approaches");
    
    // Create tools using APPROACH 4: Declarative tool! macro (at runtime)
    let approach4_subtract = tool! {
        name: "approach4_subtract",
        description: "Subtract two numbers using declarative macro",
        params: {
            minuend: f64 => "Number to subtract from",
            subtrahend: f64 => "Number to subtract",
        },
        execute: |minuend: f64, subtrahend: f64| async move {
            let result = minuend - subtrahend;
            Ok::<String, String>(format!("{} - {} = {}", minuend, subtrahend, result))
        }
    };

    let approach4_modulo = tool! {
        name: "approach4_modulo",
        description: "Calculate modulo operation with validation",
        params: {
            dividend: i32 => "Number to be divided",
            divisor: i32 => "Number to divide by",
        },
        execute: |dividend: i32, divisor: i32| async move {
            if divisor == 0 {
                Err("Division by zero not allowed".to_string())
            } else {
                let result = dividend % divisor;
                Ok::<String, String>(format!("{} % {} = {}", dividend, divisor, result))
            }
        }
    };

    // Create server with all approaches
    let server = McpServer::builder()
        .name("all_tool_approaches")
        .version("1.0.0")
        .title("All Tool Creation Approaches Example")
        .instructions("Demonstrates all available MCP tool creation approaches")
        // Approach 1: Derive macro with structured content
        .tool(Approach1MultiplyTool { a: 0.0, b: 0.0 })
        // Approach 2: Function attribute macro (no structured content yet)
        // Note: #[mcp_tool] generates a struct named Approach2PowerToolImpl
        .tool(Approach2PowerToolImpl)  
        // Approach 3: Manual trait implementation with full control
        .tool(Approach3SqrtTool)
        // Approach 4: Declarative macro (no structured content yet)
        .tool(approach4_subtract)
        .tool(approach4_modulo)
        .bind_address("127.0.0.1:8649".parse()?)
        .build()?;

    println!("\n🎯 MCP Tool Creation Approaches Comparison:");
    println!("┌─────────────────────────────────────────────────────────────────────────┐");
    println!("│ Approach                   │ Structured Content │ Complexity │ Features │");
    println!("├─────────────────────────────────────────────────────────────────────────┤");
    println!("│ 1. #[derive(McpTool)]      │        ✅          │   Simple   │  Auto    │");
    println!("│ 2. #[mcp_tool] function    │        ❌          │   Simple   │  Auto    │");
    println!("│ 3. Manual trait impl       │        ✅          │   Complex  │  Full    │");
    println!("│ 4. tool! declarative       │        ❌          │  Simplest  │  Basic   │");
    println!("└─────────────────────────────────────────────────────────────────────────┘");
    
    println!("\n📋 Available Tools:");
    println!("  🔢 approach1_multiply - Derive macro with structured content");
    println!("  ⚡ approach2_power - Function attribute macro");
    println!("  √  approach3_sqrt - Manual implementation with custom schema");
    println!("  ➖ approach4_subtract - Declarative macro");
    println!("  ➗ approach4_modulo - Declarative macro with validation");
    
    println!("\n🌐 Server starting at: http://127.0.0.1:8649/mcp");
    server.run().await?;
    Ok(())
}