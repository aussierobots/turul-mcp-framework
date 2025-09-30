// Integration test to verify README examples work
use turul_mcp_derive::{mcp_tool, mcp_resource};
use turul_mcp_server::prelude::*;
use turul_mcp_protocol::resources::ResourceContent;

// Function tool from README
#[mcp_tool(name = "add", description = "Add two numbers")]
async fn add(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<f64> {
    Ok(a + b)
}

// Resource from README
#[mcp_resource(
    uri = "file:///config.json",
    name = "config",
    description = "Application configuration"
)]
async fn get_config() -> McpResult<Vec<ResourceContent>> {
    let config = serde_json::json!({
        "app_name": "My Server",
        "version": "1.0.0"
    });

    Ok(vec![ResourceContent::blob(
        "file:///config.json",
        serde_json::to_string_pretty(&config).unwrap(),
        "application/json".to_string()
    )])
}

#[test]
fn test_readme_examples_compile() {
    // Test that we can create a server with these components
    let server = McpServer::builder()
        .name("test-server")
        .version("1.0.0")
        .tool_fn(add)
        .resource_fn(get_config)
        .bind_address("127.0.0.1:9999".parse().unwrap())
        .build();

    assert!(server.is_ok(), "Server should build successfully with README examples");
}