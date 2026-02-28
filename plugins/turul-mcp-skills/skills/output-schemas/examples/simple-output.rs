// turul-mcp-server v0.3.0
// Simple output schema — derive macro with schemars auto-detection
//
// Demonstrates the basic pattern: derive JsonSchema on output type,
// set output = Type on the tool, framework does the rest.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;

/// Output type with schemars — generates detailed JSON schema automatically.
/// Doc comments on fields become "description" in the schema.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CalculationResult {
    /// The result of the calculation
    pub value: f64,
    /// The operation that was performed
    pub operation: String,
}

/// Derive macro: output = CalculationResult is REQUIRED.
/// Without it, the schema shows {a, b, operation} (inputs) instead of {value, operation}.
#[derive(McpTool, Default)]
#[tool(
    name = "calculator_derive",
    description = "Perform calculations with automatic schema generation",
    output = CalculationResult
)]
pub struct CalculatorDeriveTool {
    #[param(description = "First number")]
    pub a: f64,
    #[param(description = "Second number")]
    pub b: f64,
    #[param(description = "Operation: add, subtract, multiply, divide")]
    pub operation: String,
}

impl CalculatorDeriveTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<CalculationResult> {
        let value = match self.operation.as_str() {
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
                    "Invalid operation",
                ));
            }
        };

        Ok(CalculationResult {
            value,
            operation: self.operation.clone(),
        })
    }
}

/// Function macro: return type auto-detected — no output attribute needed.
#[turul_mcp_derive::mcp_tool(
    name = "calculator_function",
    description = "Perform addition with automatic schema generation"
)]
async fn add_numbers(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<CalculationResult> {
    Ok(CalculationResult {
        value: a + b,
        operation: "add".to_string(),
    })
}
