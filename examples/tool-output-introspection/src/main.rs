//! Tool Output Introspection Example
//!
//! Demonstrates automatic output schema generation using struct field introspection.
//!
//! This example shows how the framework automatically generates DETAILED output schemas
//! by analyzing struct fields at compile time - NO schemars dependency required!
//!
//! **KEY LIMITATION**: Introspection ONLY works when the tool returns Self.
//! When you specify `output = SomeOtherType`, the macro cannot introspect that external type.
//! For external types, use the schemars approach (see tool-output-schemas example).
//!
//! The introspection approach works for:
//! - Tools that return Self (the struct being derived)
//! - Simple field types: String, numbers, bool, Option<T>
//! - NO schemars dependency needed!

use serde::{Deserialize, Serialize};
use turul_mcp_derive::McpTool;
use turul_mcp_protocol::McpResult;
use turul_mcp_server::prelude::*;

/// Calculator tool that returns SELF - enabling full struct introspection!
///
/// When you DON'T specify `output = Type`, the framework assumes you return Self
/// and automatically introspects the struct fields for a detailed schema.
/// NO schemars dependency needed!
#[derive(McpTool, Default, Clone, Debug, Serialize, Deserialize)]
#[tool(
    name = "calculator_introspect",
    description = "Calculator with struct field introspection - returns Self for detailed schema"
)]
pub struct Calculator {
    /// First number in the calculation
    #[param(description = "First number")]
    pub a: f64,

    /// Second number in the calculation
    #[param(description = "Second number")]
    pub b: f64,

    /// The operation to perform
    #[param(description = "Operation: add, subtract, multiply, divide")]
    pub operation: String,

    /// The calculated result (populated after execution)
    pub result: f64,

    /// Optional calculation message
    pub message: Option<String>,
}

impl Calculator {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<Self> {
        let result = match self.operation.as_str() {
            "add" => self.a + self.b,
            "subtract" => self.a - self.b,
            "multiply" => self.a * self.b,
            "divide" => {
                if self.b == 0.0 {
                    return Err(turul_mcp_protocol::McpError::tool_execution(
                        "Cannot divide by zero",
                    ));
                }
                self.a / self.b
            }
            _ => {
                return Err(turul_mcp_protocol::McpError::tool_execution(
                    "Invalid operation. Use: add, subtract, multiply, divide",
                ));
            }
        };

        Ok(Self {
            a: self.a,
            b: self.b,
            operation: self.operation.clone(),
            result,
            message: Some(format!(
                "{} {} {} = {}",
                self.a, self.operation, self.b, result
            )),
        })
    }
}

/// Temperature converter that returns Self - demonstrating introspection with different field types
#[derive(McpTool, Default, Clone, Debug, Serialize, Deserialize)]
#[tool(
    name = "temp_converter",
    description = "Convert between Celsius and Fahrenheit - showcases introspection"
)]
pub struct TempConverter {
    /// Input temperature value
    #[param(description = "Temperature value to convert")]
    pub value: f64,

    /// Input unit (C or F)
    #[param(description = "Input unit: C for Celsius, F for Fahrenheit")]
    pub from_unit: String,

    /// Output temperature in the other unit
    pub converted_value: f64,

    /// Output unit
    pub to_unit: String,

    /// Whether the conversion was successful
    pub success: bool,
}

impl TempConverter {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<Self> {
        let (converted, to_unit) = match self.from_unit.to_uppercase().as_str() {
            "C" => ((self.value * 9.0 / 5.0) + 32.0, "F".to_string()),
            "F" => ((self.value - 32.0) * 5.0 / 9.0, "C".to_string()),
            _ => {
                return Err(turul_mcp_protocol::McpError::tool_execution(
                    "Invalid unit. Use 'C' or 'F'",
                ));
            }
        };

        Ok(Self {
            value: self.value,
            from_unit: self.from_unit.clone(),
            converted_value: converted,
            to_unit,
            success: true,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Create server with tools using struct introspection
    let server = McpServer::builder()
        .name("tool-output-introspection-server")
        .version("0.1.0")
        .tool(Calculator::default())
        .tool(TempConverter::default())
        .build()?;

    tracing::info!("Starting MCP server with struct introspection for output schemas...");
    tracing::info!("Both tools return Self - enabling DETAILED schema introspection!");
    tracing::info!("NO schemars dependency - schemas generated from struct field analysis");
    tracing::info!("");
    tracing::info!("Try calling tools/list to see detailed output schemas!");
    tracing::info!("Each field (result, message, converted_value, etc.) is listed with its type");

    Ok(server.run().await?)
}
