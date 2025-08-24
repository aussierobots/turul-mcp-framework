use mcp_derive::McpTool;
use mcp_server::{McpResult, McpServer};
use tracing::info;

#[derive(McpTool, Clone)]
#[tool(name = "calculator_add_simple", description = "Add two numbers simple")]
#[output_type(f64)]
struct CalculatorAddSimpleTool {
    #[param(description = "First number")]
    a: f64,
    #[param(description = "Second number")]
    b: f64,
}

impl CalculatorAddSimpleTool {
    async fn execute(&self) -> McpResult<f64> {
        Ok(self.a + self.b)
    }
}

// #[derive(Debug, Clone, Serialize)]
// struct AdditionResult {
//     result: f64,
// }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting calculator_add_simple server");
    let server = McpServer::builder()
        .name("calculator_add_simple")
        .version("0.0.1")
        .title("Calculator Add Simple Server")
        .instructions("Add two numbers simple")
        .tool(CalculatorAddSimpleTool { a: 0.0, b: 0.0 })
        .bind_address("127.0.0.1:8645".parse()?)
        .build()?;

    server.run().await?;
    Ok(())
}
