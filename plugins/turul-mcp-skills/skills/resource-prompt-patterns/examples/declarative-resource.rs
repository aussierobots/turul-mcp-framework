// turul-mcp-server v0.3
// Resource Pattern 3: Declarative Macro resource!{}
// Generates a full struct + all traits + McpResource impl from a closure.
// Good for inline one-off resources.

use turul_mcp_server::prelude::*;

fn build_server() -> Result<McpServer, Box<dyn std::error::Error>> {
    let config = resource! {
        uri: "file:///config.json",
        name: "app_config",
        description: "Current application configuration",
        content: |_params, _session| async move {
            let config = serde_json::json!({
                "version": "1.0.0",
                "debug": false,
                "maxConnections": 100
            });
            Ok(vec![ResourceContent::text(
                "file:///config.json",
                serde_json::to_string_pretty(&config).unwrap(),
            )])
        }
    };

    let server = McpServer::builder()
        .name("config-server")
        .resource(config)   // .resource() for declarative macro
        .build()?;
    Ok(server)
}
