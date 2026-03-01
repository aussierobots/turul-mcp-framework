// turul-mcp-server v0.3
// Execution errors: tool_execution, .map_err(), and ? operator

use turul_mcp_derive::mcp_tool;
use turul_mcp_protocol::McpError;
use turul_mcp_server::McpResult;

// Pattern 1: Wrapping external errors with .map_err()
#[mcp_tool(name = "fetch_url", description = "Fetch content from a URL")]
async fn fetch_url(
    #[param(description = "URL to fetch")] url: String,
) -> McpResult<String> {
    let response = reqwest::get(&url)
        .await
        .map_err(|e| McpError::tool_execution(&format!("HTTP request failed: {e}")))?;

    let text = response
        .text()
        .await
        .map_err(|e| McpError::tool_execution(&format!("Failed to read response: {e}")))?;

    Ok(text)
}

// Pattern 2: Using ? with types that have From impls
#[mcp_tool(name = "read_config", description = "Read JSON config file")]
async fn read_config(
    #[param(description = "Path to config")] path: String,
) -> McpResult<serde_json::Value> {
    // io::Error has From impl → McpError::IoError automatically
    let content = std::fs::read_to_string(&path)?;

    // serde_json::Error has From impl → McpError::SerializationError automatically
    let config: serde_json::Value = serde_json::from_str(&content)?;

    Ok(config)
}

// Pattern 3: Quick string conversion (only in tool handlers)
#[mcp_tool(name = "process", description = "Process data")]
async fn process(
    #[param(description = "Input data")] data: String,
) -> McpResult<String> {
    if data.len() > 10_000 {
        // From<&str> → ToolExecutionError (-32010)
        return Err("Input too large (max 10,000 chars)".into());
    }
    Ok(data.to_uppercase())
}

// Pattern 4: Resource handler — use resource_execution, NOT string conversion
// async fn read_resource(uri: &str) -> McpResult<String> {
//     std::fs::read_to_string(uri)
//         .map_err(|e| McpError::resource_execution(&e.to_string()))
// }
