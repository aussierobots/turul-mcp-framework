// turul-mcp-server v0.3.0
// Level 2: Derive Macro (#[derive(McpTool)]) — session access + custom output
//
// Source: examples/calculator-add-simple-server-derive/src/main.rs

use serde::{Deserialize, Serialize};
use tracing::info;
use turul_mcp_derive::McpTool;
use turul_mcp_server::{McpResult, McpServer};

/// Custom output type. Schemars auto-detected when derived.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct AdditionResult {
    sum: f64,
}

/// Derive McpTool on a struct. The `output` attribute is REQUIRED
/// for non-primitive return types — without it, the schema shows
/// the input fields instead of the output type.
#[derive(McpTool, Clone)]
#[tool(
    name = "calculator_add_derive",
    description = "Add two numbers using derive macro (Level 2)",
    output = AdditionResult
)]
struct CalculatorAddDeriveTool {
    #[param(description = "First number")]
    a: f64,
    #[param(description = "Second number")]
    b: f64,
}

/// The execute method receives an optional SessionContext.
/// This is the derive macro's main advantage over function macros.
impl CalculatorAddDeriveTool {
    async fn execute(
        &self,
        _session: Option<turul_mcp_server::SessionContext>,
    ) -> McpResult<AdditionResult> {
        Ok(AdditionResult {
            sum: self.a + self.b,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting calculator_add_derive server");
    let server = McpServer::builder()
        .name("calculator_add_derive")
        .version("0.0.1")
        .title("Calculator Add Derive Server")
        .instructions("Add two numbers using derive macro (Level 2)")
        .tool(CalculatorAddDeriveTool { a: 0.0, b: 0.0 }) // Use .tool() for derive macros
        .build()?;

    server.run().await?;
    Ok(())
}
