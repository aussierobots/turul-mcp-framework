//! # Resource Macro Example
//!
//! This example demonstrates the `resource!` declarative macro for creating simple MCP resources
//! with inline content generation closures.

use mcp_derive::resource;
use mcp_server::{McpServer, McpResource};
use mcp_protocol::resources::ResourceContent;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("Testing resource! declarative macro...");

    // Create resources using the resource! macro
    let config_resource = resource! {
        uri: "file://config.json",
        name: "Application Configuration",
        description: "Main application configuration file",
        content: |_self| async move {
            let config = serde_json::json!({
                "app_name": "Resource Macro Example",
                "version": "1.0.0",
                "debug": true,
                "features": ["resources", "declarative_macros"]
            });
            Ok(vec![ResourceContent::blob(
                serde_json::to_string_pretty(&config).unwrap(),
                "application/json".to_string()
            )])
        }
    };

    let status_resource = resource! {
        uri: "system://status",
        name: "System Status",
        description: "Current system health and status information",
        content: |_self| async move {
            let status = format!(
                "System Status: OK\nUptime: {} seconds\nMemory: 512MB\nCPU: 25%",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() % 3600
            );
            Ok(vec![ResourceContent::text(status)])
        }
    };

    let log_resource = resource! {
        uri: "file://app.log",
        name: "Application Log",
        description: "Recent application log entries",
        content: |_self| async move {
            let log_entries = vec![
                "2024-01-01 12:00:00 INFO  Resource macro example started",
                "2024-01-01 12:00:01 INFO  Configuration loaded successfully",
                "2024-01-01 12:00:02 INFO  MCP resources initialized",
                "2024-01-01 12:00:03 INFO  Server ready to accept connections",
                "2024-01-01 12:00:10 DEBUG Resource accessed: config.json",
            ];
            Ok(vec![ResourceContent::text(log_entries.join("\n"))])
        }
    };

    // Test the resources
    println!("\nTesting resources created with resource! macro:");
    
    println!("\n1. Testing config resource:");
    println!("   URI: {}", config_resource.uri());
    println!("   Name: {}", config_resource.name());
    println!("   Description: {}", config_resource.description());
    match config_resource.read(None).await {
        Ok(content) => {
            for (i, item) in content.iter().enumerate() {
                println!("   Content {}: {:?}", i + 1, item);
            }
        }
        Err(e) => println!("   Error: {}", e),
    }

    println!("\n2. Testing status resource:");
    println!("   URI: {}", status_resource.uri());
    println!("   Name: {}", status_resource.name());
    println!("   Description: {}", status_resource.description());
    match status_resource.read(None).await {
        Ok(content) => {
            for (i, item) in content.iter().enumerate() {
                println!("   Content {}: {:?}", i + 1, item);
            }
        }
        Err(e) => println!("   Error: {}", e),
    }

    println!("\n3. Testing log resource:");
    println!("   URI: {}", log_resource.uri());
    println!("   Name: {}", log_resource.name());
    println!("   Description: {}", log_resource.description());
    match log_resource.read(None).await {
        Ok(content) => {
            for (i, item) in content.iter().enumerate() {
                println!("   Content {}: {:?}", i + 1, item);
            }
        }
        Err(e) => println!("   Error: {}", e),
    }

    // Create and run a simple server with these resources
    let server = McpServer::builder()
        .name("resource-macro-example")
        .version("1.0.0")
        .title("Resource Macro Example Server")
        .instructions("This server demonstrates the resource! declarative macro.")
        .resource(config_resource)
        .resource(status_resource)
        .resource(log_resource)
        .with_resources()
        .bind_address("127.0.0.1:8011".parse()?)
        .build()?;

    println!("\nStarting server at: http://127.0.0.1:8011/mcp");
    println!("Resources available:");
    println!("  - file://config.json: Application configuration");
    println!("  - system://status: System status information");
    println!("  - file://app.log: Application log entries");

    server.run().await?;
    Ok(())
}