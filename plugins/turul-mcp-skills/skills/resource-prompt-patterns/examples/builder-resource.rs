// turul-mcp-server v0.3
// Resource Pattern 4: ResourceBuilder
// For resources whose definitions are loaded at runtime (config, DB, etc.).

use turul_mcp_server::prelude::*;

fn build_server() -> Result<McpServer, Box<dyn std::error::Error>> {
    // Static content resource
    let readme = ResourceBuilder::new("file:///readme.txt")
        .name("readme")
        .description("Project README")
        .mime_type("text/plain")
        .text_content("Welcome to the project!")
        .build()?;

    // Dynamic content resource — .read_text() callback receives the URI
    let status = ResourceBuilder::new("file:///status.json")
        .name("status")
        .description("Live system status")
        .mime_type("application/json")
        .read_text(|_uri| async move {
            let uptime = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            Ok(format!(r#"{{"status":"ok","uptime":{uptime}}}"#))
        })
        .build()?;

    let server = McpServer::builder()
        .name("dynamic-server")
        .resource(readme)
        .resource(status)
        .build()?;
    Ok(server)
}
