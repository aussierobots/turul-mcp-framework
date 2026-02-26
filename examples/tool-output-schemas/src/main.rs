//! Tool Output Schemas Example
//!
//! Demonstrates automatic output schema generation using schemars.
//!
//! This example shows how to use the `schemars` attribute to automatically
//! generate tool output schemas from Rust types with JsonSchema derive.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use turul_mcp_derive::{McpTool, mcp_tool};
use turul_mcp_protocol::McpResult;
use turul_mcp_server::prelude::*;

/// Output type for calculator operations
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CalculationResult {
    /// The result of the calculation
    pub value: f64,
    /// The operation that was performed
    pub operation: String,
}

/// Calculator tool using derive macro with automatic schemars detection
#[derive(McpTool, Default)]
#[tool(
    name = "calculator_derive",
    description = "Perform calculations with automatic schema generation",
    output = CalculationResult
    // Schemars automatically detected from CalculationResult's #[derive(JsonSchema)]!
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

/// Calculator tool using function macro with automatic schemars detection
#[mcp_tool(
    name = "calculator_function",
    description = "Perform addition with automatic schema generation"
    // Schemars automatically detected from CalculationResult's #[derive(JsonSchema)]!
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

// ===== NESTED SCHEMA EXAMPLE =====

/// Statistics for a dataset
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Statistics {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub count: usize,
}

/// Individual data point in a series
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DataPoint {
    pub timestamp: String,
    pub value: f64,
}

/// Analysis result with nested objects and arrays
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalysisResult {
    /// Name of the dataset analyzed
    pub dataset: String,
    /// Summary statistics (nested object)
    pub stats: Statistics,
    /// Individual data points (array of objects)
    pub points: Vec<DataPoint>,

    // ✅ BEST PRACTICE: Optional fields should use skip_serializing_if
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Tool demonstrating nested schemas and optional fields
#[derive(McpTool, Default)]
#[tool(
    name = "analyze_data",
    description = "Analyze a dataset with nested output schema",
    output = AnalysisResult
)]
pub struct AnalyzeDataTool {
    #[param(description = "Dataset name")]
    pub dataset: String,
    #[param(description = "Comma-separated values")]
    pub values: String,
}

impl AnalyzeDataTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<AnalysisResult> {
        let values: Vec<f64> = self
            .values
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();

        if values.is_empty() {
            return Err(turul_mcp_protocol::McpError::tool_execution(
                "No valid values provided",
            ));
        }

        let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let mean = values.iter().sum::<f64>() / values.len() as f64;

        let points = values
            .iter()
            .enumerate()
            .map(|(i, &v)| DataPoint {
                timestamp: format!("T{}", i),
                value: v,
            })
            .collect();

        Ok(AnalysisResult {
            dataset: self.dataset.clone(),
            stats: Statistics {
                min,
                max,
                mean,
                count: values.len(),
            },
            points,
            warning: if values.len() < 3 {
                Some("Small sample size".to_string())
            } else {
                None // Omitted from JSON, not serialized as null
            },
            notes: None, // Omitted from JSON
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

    // Create server with tools that use schemars
    let server = McpServer::builder()
        .name("tool-output-schemas-server")
        .version("0.1.0")
        .tool(CalculatorDeriveTool::default())
        .tool(add_numbers())
        .tool(AnalyzeDataTool::default())
        .build()?;

    tracing::info!("Starting MCP server with automatic output schemas...");
    tracing::info!("Examples:");
    tracing::info!("  - calculator_derive: Basic flat schema (value, operation)");
    tracing::info!("  - calculator_function: Function macro with schema");
    tracing::info!("  - analyze_data: Nested schemas + arrays + optional fields");
    tracing::info!("");
    tracing::info!("✅ All tools use schemars for automatic schema generation");
    tracing::info!("✅ Demonstrates nested objects ($ref resolution)");
    tracing::info!("✅ Demonstrates arrays of objects (detailed item schemas)");
    tracing::info!("✅ Demonstrates optional fields (skip_serializing_if pattern)");

    Ok(server.run().await?)
}
