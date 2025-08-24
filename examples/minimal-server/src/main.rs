//! # Minimal MCP Server Example
//!
//! This example demonstrates the absolute minimum setup for an MCP server.
//! It creates a simple echo tool that demonstrates basic MCP functionality.

use std::collections::HashMap;
use std::net::SocketAddr;

use async_trait::async_trait;
use mcp_server::{McpServer, McpTool, SessionContext};
use mcp_protocol::{ToolSchema, ToolResult, schema::JsonSchema, McpResult};
// use mcp_protocol::McpError; // TODO: Use for error handling
use serde_json::Value;

/// Simple echo tool - demonstrates the minimal McpTool implementation
struct EchoTool;

#[async_trait]
impl McpTool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "Echo back the input text"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("text".to_string(), JsonSchema::string_with_description("Text to echo back")),
            ]))
            .with_required(vec!["text".to_string()])
    }

    fn output_schema(&self) -> Option<ToolSchema> {
        Some(ToolSchema::object()
            .with_properties(HashMap::from([
                ("echo_response".to_string(), JsonSchema::string_with_description("The echoed text with 'Echo: ' prefix")),
            ]))
            .with_required(vec!["echo_response".to_string()]))
    }

    async fn call(&self, args: Value, _session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let text = args.get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("No text provided");

        let echo_response = format!("Echo: {}", text);
        
        // Return structured data matching the output schema
        let response_data = serde_json::json!({
            "echo_response": echo_response
        });
        
        Ok(vec![ToolResult::resource(response_data)])
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("Starting Minimal MCP Server");

    // Create the simplest possible MCP server
    let server = McpServer::builder()
        .name("minimal-server")           // Required: Server name
        .version("1.0.0")               // Required: Server version  
        .tool(EchoTool)                 // Add one tool
        .bind_address("127.0.0.1:8641".parse::<SocketAddr>().unwrap()) // Use port 8641
        .build()?;                      // Build with defaults

    println!("MCP server running at: http://127.0.0.1:8641/mcp");
    println!("Try this curl command:");
    println!(r#"curl -X POST http://127.0.0.1:8641/mcp \
  -H "Content-Type: application/json" \
  -d '{{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {{
      "protocolVersion": "2025-06-18",
      "capabilities": {{}},
      "clientInfo": {{"name": "test-client", "version": "1.0.0"}}
    }}
  }}'"#);

    // Run the server (blocks until interrupted)
    server.run().await?;

    Ok(())
}