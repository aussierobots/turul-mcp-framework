use async_trait::async_trait;
use mcp_protocol::{CallToolResponse, ToolResult, ToolSchema};
use mcp_server::{McpResult, McpServer, McpTool, SessionContext};
use serde::Serialize;
use serde_json::Value;
use tracing::info;

/// Manual implementation of a calculator tool without derive macros
#[derive(Clone)]
struct CalculatorAddTool {
    a: f64,
    b: f64,
}

impl CalculatorAddTool {
    fn new() -> Self {
        Self { a: 0.0, b: 0.0 }
    }
    
    async fn execute(&self, a: f64, b: f64) -> McpResult<f64> {
        Ok(a + b)
    }
}

#[async_trait]
impl McpTool for CalculatorAddTool {
    fn name(&self) -> &str {
        "calculator_add_manual"
    }

    fn description(&self) -> &str {
        "Add two numbers (manual implementation)"
    }

    fn input_schema(&self) -> ToolSchema {
        use mcp_protocol::schema::JsonSchema;
        use std::collections::HashMap;
        
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("a".to_string(), JsonSchema::number().with_description("First number")),
                ("b".to_string(), JsonSchema::number().with_description("Second number")),
            ]))
            .with_required(vec!["a".to_string(), "b".to_string()])
    }
    
    fn output_schema(&self) -> Option<ToolSchema> {
        use mcp_protocol::schema::JsonSchema;
        use std::collections::HashMap;
        
        Some(ToolSchema::object()
            .with_properties(HashMap::from([
                ("value".to_string(), JsonSchema::number())
            ]))
            .with_required(vec!["value".to_string()]))
    }

    async fn call(&self, args: Value, _session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        // Extract parameters manually
        let a = args.get("a")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| mcp_protocol::McpError::invalid_param_type("a", "number", "other"))?;
        
        let b = args.get("b")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| mcp_protocol::McpError::invalid_param_type("b", "number", "other"))?;

        // Execute calculation
        let result = self.execute(a, b).await?;
        
        // Return JSON text matching structured content
        let json_text = serde_json::json!({"value": result}).to_string();
        Ok(vec![ToolResult::text(json_text)])
    }
    
    async fn execute(&self, args: Value, session: Option<SessionContext>) -> McpResult<CallToolResponse> {
        // Get content from call method
        let content = self.call(args.clone(), session).await?;
        let response = CallToolResponse::success(content);
        
        // Add structured content
        if self.output_schema().is_some() {
            // Extract parameters again for structured content
            let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
            
            match self.execute(a, b).await {
                Ok(result) => {
                    let structured_content = serde_json::json!({"value": result});
                    Ok(response.with_structured_content(structured_content))
                }
                Err(_) => Ok(response), // If structured content fails, return without it
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

    info!("Starting calculator_add_manual server");
    let server = McpServer::builder()
        .name("calculator_add_manual")
        .version("0.0.1")
        .title("Calculator Add Manual Server")
        .instructions("Add two numbers using manual trait implementation")
        .tool(CalculatorAddTool::new())
        .bind_address("127.0.0.1:8646".parse()?)
        .build()?;

    server.run().await?;
    Ok(())
}