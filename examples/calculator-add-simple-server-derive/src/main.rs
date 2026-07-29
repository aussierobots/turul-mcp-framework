use serde::{Deserialize, Serialize};
use tracing::info;
use turul_mcp_derive::McpTool;
use turul_mcp_server::{McpResult, McpServer};

/// The sum of the two operands. schemars carries this doc comment into the
/// tool's `outputSchema` as the type's `description`.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct AdditionResult {
    sum: f64,
}

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
        .version("0.4.0")
        .title("Calculator Add Derive Server")
        .instructions("Add two numbers using derive macro (Level 2)")
        .tool(CalculatorAddDeriveTool { a: 0.0, b: 0.0 })
        .bind_address("127.0.0.1:8647".parse()?)
        .build()?;

    server.run().await?;
    Ok(())
}
