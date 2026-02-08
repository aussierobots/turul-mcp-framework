//! # All Tool Creation Approaches Example
//!
//! This example demonstrates ALL FOUR ways to create MCP tools:
//! 1. Derive Macro - Struct-based with `#[derive(McpTool)]`
//! 2. Function Macro - Function-based with `#[mcp_tool]`
//! 3. Declarative Macro - Inline with `tool!{}`
//! 4. Manual Implementation - Full control with trait implementations
//!
//! Use this to understand the trade-offs of each approach.

use async_trait::async_trait;
use turul_mcp_derive::{mcp_tool, tool, McpTool};
use turul_mcp_protocol::{ToolResult, ToolSchema, schema::JsonSchema, tools::*};
use turul_mcp_builders::prelude::HasIcon;
use turul_mcp_server::{McpResult, McpServer, McpTool, SessionContext};
use serde_json::Value;
use std::collections::HashMap;

// =============================================================================
// APPROACH 1: DERIVE MACRO - Structured with automatic trait implementation
// =============================================================================
// Best for: Tools with complex state, structured input/output
// Pros: Type-safe parameters, automatic validation, structured results
// Cons: More boilerplate for simple tools

#[derive(McpTool, Clone)]
#[tool(name = "approach1_multiply", description = "Multiply two numbers using derive macro",
    output = f64
)]
struct MultiplyTool {
    #[param(description = "First number")]
    a: f64,
    #[param(description = "Second number")]
    b: f64,
}

impl MultiplyTool {
    async fn execute(&self) -> McpResult<f64> {
        Ok(self.a * self.b)
    }
}

// =============================================================================
// APPROACH 2: FUNCTION MACRO - Simple function with automatic conversion
// =============================================================================
// Best for: Simple stateless tools, quick prototypes
// Pros: Minimal code, looks like normal functions, automatic schema generation
// Cons: Less control over schema details

#[mcp_tool(name = "approach2_power", description = "Calculate power using function macro")]
async fn calculate_power(
    #[param(description = "Base number")] base: f64,
    #[param(description = "Exponent")] exp: f64,
) -> McpResult<f64> {
    Ok(base.powf(exp))
}

// =============================================================================
// APPROACH 3: DECLARATIVE MACRO - Inline tool creation
// =============================================================================
// Best for: Quick tools, scripting-style development, dynamic tools
// Pros: Everything in one place, closure-based logic, very concise
// Cons: No compile-time type checking of parameters

fn create_sqrt_tool() -> impl McpTool {
    tool! {
        name: "approach3_sqrt",
        description: "Calculate square root using declarative macro",
        params: {
            number: f64 => "Number to calculate square root of",
        },
        execute: |number: f64| async move {
            if number < 0.0 {
                Err("Cannot calculate square root of negative number")
            } else {
                Ok(format!("√{} = {}", number, number.sqrt()))
            }
        }
    }
}

// =============================================================================
// APPROACH 4: MANUAL IMPLEMENTATION - Full control over every aspect
// =============================================================================
// Best for: Complex tools, custom validation, special requirements
// Pros: Complete control, can optimize performance, custom behaviors
// Cons: Most verbose, must implement all traits manually

#[derive(Clone)]
struct DivideTool {
    input_schema: ToolSchema,
}

impl DivideTool {
    fn new() -> Self {
        let input_schema = ToolSchema::object()
            .with_properties(HashMap::from([
                ("dividend".to_string(), JsonSchema::number().with_description("Number to divide")),
                ("divisor".to_string(), JsonSchema::number().with_description("Number to divide by")),
            ]))
            .with_required(vec!["dividend".to_string(), "divisor".to_string()]);
        Self { input_schema }
    }
}

// Manual implementation of all fine-grained traits
impl HasBaseMetadata for DivideTool {
    fn name(&self) -> &str {
        "approach4_divide"
    }
}

impl HasDescription for DivideTool {
    fn description(&self) -> Option<&str> {
        Some("Divide two numbers with validation using manual implementation")
    }
}

impl HasInputSchema for DivideTool {
    fn input_schema(&self) -> &ToolSchema {
        &self.input_schema
    }
}

impl HasOutputSchema for DivideTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}

impl HasAnnotations for DivideTool {
    fn annotations(&self) -> Option<&ToolAnnotations> {
        None
    }
}

impl HasToolMeta for DivideTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

impl HasIcons for DivideTool {}

#[async_trait]
impl McpTool for DivideTool {
    async fn call(&self, args: Value, _session: Option<SessionContext>) -> McpResult<CallToolResult> {
        let dividend = args.get("dividend")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| turul_mcp_protocol::McpError::missing_param("dividend"))?;

        let divisor = args.get("divisor")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| turul_mcp_protocol::McpError::missing_param("divisor"))?;

        // Custom validation logic
        if divisor == 0.0 {
            return Err(turul_mcp_protocol::McpError::InvalidParameters("Division by zero".to_string()));
        }

        let result = dividend / divisor;

        Ok(CallToolResult {
            content: vec![ToolResult::text(format!("{} ÷ {} = {}", dividend, divisor, result))],
            is_error: None,
            structured_content: None,
            meta: None,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🎯 All Tool Creation Approaches Example");
    println!("========================================");
    println!();
    println!("This server demonstrates 4 different ways to create MCP tools:");
    println!("1. Derive Macro    - Structured with #[derive(McpTool)]");
    println!("2. Function Macro  - Simple with #[mcp_tool]");
    println!("3. Declarative     - Inline with tool!{{}}");
    println!("4. Manual          - Full control with trait implementations");
    println!();

    let server = McpServer::builder()
        .name("all-approaches-demo")
        .version("1.0.0")
        .title("All Tool Creation Approaches Demo")
        .instructions("Compare different ways to create MCP tools. Each approach has different trade-offs.")

        // Approach 1: Derive macro (requires instantiation)
        .tool(MultiplyTool { a: 0.0, b: 0.0 }) // Values will be replaced at runtime

        // Approach 2: Function macro
        .tool_fn(calculate_power)

        // Approach 3: Declarative macro
        .tool(create_sqrt_tool())

        // Approach 4: Manual implementation
        .tool(DivideTool::new())

        .bind_address("127.0.0.1:8650".parse()?)
        .build()?;

    println!("📡 Server running at: http://127.0.0.1:8650/mcp");
    println!();
    println!("Available tools:");
    println!("  • approach1_multiply - Derive macro approach");
    println!("  • approach2_power    - Function macro approach");
    println!("  • approach3_sqrt     - Declarative macro approach");
    println!("  • approach4_divide   - Manual implementation approach");
    println!();
    println!("Try each tool to see how different approaches work!");

    server.run().await?;
    Ok(())
}