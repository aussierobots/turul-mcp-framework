// turul-mcp-server v0.3
// Resource Pattern 1: Function Macro #[mcp_resource]
// URI template variable {service} becomes a typed function parameter.

use turul_mcp_derive::mcp_resource;
use turul_mcp_server::prelude::*;

#[mcp_resource(
    uri = "file:///logs/{service}.log",
    name = "service_log",
    description = "Recent log entries for a named service",
    mime_type = "text/plain"
)]
async fn service_log(service: String) -> McpResult<Vec<ResourceContent>> {
    let path = format!("/var/log/{service}.log");
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| McpError::tool_execution(format!("Cannot read {path}: {e}")))?;

    // Return the last 100 lines
    let tail: String = content.lines().rev().take(100).collect::<Vec<_>>()
        .into_iter().rev().collect::<Vec<_>>().join("\n");

    Ok(vec![ResourceContent::text(
        &format!("file:///logs/{service}.log"),
        tail,
    )])
}

// Registration: use .resource_fn() for function macros
fn build_server() -> Result<McpServer, Box<dyn std::error::Error>> {
    let server = McpServer::builder()
        .name("log-reader")
        .resource_fn(service_log)
        .build()?;
    Ok(server)
}
