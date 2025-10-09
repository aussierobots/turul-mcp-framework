//! Minimal Echo Tool Lambda MCP Server Example
//!
//! This example demonstrates the simplest possible Lambda MCP server with:
//! - Built-in ping tool (from framework)
//! - Custom echo tool using derive macro
//!
//! Usage:
//! ```bash
//! # Build and deploy to Lambda
//! cargo lambda build --package turul-mcp-aws-lambda --example lambda-echo-server
//! cargo lambda deploy --package turul-mcp-aws-lambda --example lambda-echo-server
//!
//! # Or run locally for testing (requires cargo-lambda)
//! cargo lambda watch --package turul-mcp-aws-lambda --example lambda-echo-server
//! ```

use lambda_http::{run_with_streaming_response, service_fn};
use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
use turul_mcp_derive::McpTool;
use turul_mcp_server::{McpResult, SessionContext};

/// Simple echo tool that returns whatever message is sent to it
#[derive(McpTool, Clone, Default)]
#[tool(name = "echo", description = "Echo back the provided message",
    output = String
)]
struct EchoTool {
    #[param(description = "Message to echo back")]
    message: String,
}

impl EchoTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok(format!("Echo: {}", self.message))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing with RUST_LOG environment variable
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .without_time()
        .init();

    // Create Lambda MCP server with echo tool
    let server = LambdaMcpServerBuilder::new()
        .name("echo-lambda-server")
        .version("1.0.0")
        .tool(EchoTool::default()) // Add our echo tool
        .sse(true) // Enable SSE streaming
        .cors_allow_all_origins() // Allow CORS for browser clients
        .build()
        .await?;

    // Create handler for Lambda runtime
    let handler = server.handler().await?;

    tracing::info!("🚀 Echo Lambda MCP server ready!");
    tracing::info!("📋 Available tools: ping (built-in), echo (custom)");

    // Run with Lambda streaming response support
    run_with_streaming_response(service_fn(move |req| {
        let handler = handler.clone();
        async move {
            handler
                .handle_streaming(req) // Use handle_streaming for proper SSE support
                .await
        }
    }))
    .await
}
