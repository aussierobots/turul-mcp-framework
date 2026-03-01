// turul-mcp-client v0.3
// Minimal MCP client: connect, list tools, call a tool, disconnect.

use serde_json::json;
use turul_mcp_client::{McpClientBuilder, McpClientResult};

#[tokio::main]
async fn main() -> McpClientResult<()> {
    // Build client — transport auto-detected from URL
    let client = McpClientBuilder::new()
        .with_url("http://localhost:8080/mcp")?
        .build();

    // Connect performs the MCP initialize handshake
    client.connect().await?;
    println!("Connected: {}", client.is_ready().await);

    // List available tools
    let tools = client.list_tools().await?;
    for tool in &tools {
        println!("Tool: {} — {}", tool.name, tool.description.as_deref().unwrap_or(""));
    }

    // Call a tool
    let result = client
        .call_tool("add", json!({"a": 5.0, "b": 3.0}))
        .await?;
    println!("Result: {result:?}");

    // List resources
    let resources = client.list_resources().await?;
    for r in &resources {
        println!("Resource: {} ({})", r.name, r.uri);
    }

    // Read a resource
    if let Some(r) = resources.first() {
        let contents = client.read_resource(&r.uri).await?;
        println!("Content: {contents:?}");
    }

    // List prompts
    let prompts = client.list_prompts().await?;
    for p in &prompts {
        println!("Prompt: {}", p.name);
    }

    // Ping
    client.ping().await?;
    println!("Server is alive");

    // Clean disconnect (sends DELETE to server)
    client.disconnect().await?;
    Ok(())
}
