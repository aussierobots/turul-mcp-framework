use turul_mcp_server::{McpServer, ToolBuilder};
use serde_json::json;
use tracing::info;

/// Level 3: Builder Pattern (Runtime Flexibility)
/// Construct tools at runtime using builder pattern - great for dynamic/configurable tools
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting calculator_add_builder server");
    
    // Level 3: Build tool at runtime using builder pattern
    let add_tool = ToolBuilder::new("calculator_add_builder")
        .description("Add two numbers using builder pattern (Level 3)")
        .number_param("a", "First number")
        .number_param("b", "Second number")
        .number_output() // Generates {"result": number} schema
        .execute(|args| async move {
            let a = args.get("a").and_then(|v| v.as_f64())
                .ok_or("Missing or invalid parameter 'a'")?;
            let b = args.get("b").and_then(|v| v.as_f64())
                .ok_or("Missing or invalid parameter 'b'")?;
            
            let sum = a + b;
            Ok(json!({"result": sum}))
        })
        .build()
        .map_err(|e| format!("Failed to build tool: {}", e))?;
    
    let server = McpServer::builder()
        .name("calculator_add_builder")
        .version("0.0.1")
        .title("Calculator Add Builder Server")
        .instructions("Add two numbers using builder pattern (Level 3 - Runtime Flexibility)")
        .tool(add_tool)
        .bind_address("127.0.0.1:8649".parse()?)
        .build()?;

    server.run().await?;
    Ok(())
}