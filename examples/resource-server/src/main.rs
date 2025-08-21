//! # Resource Server Example
//!
//! This example demonstrates using the #[derive(McpResource)] macro to create
//! MCP resources with minimal boilerplate code.

use serde::{Serialize, Deserialize};
use mcp_derive::McpResource;
use mcp_server::{McpServer, McpResource};
// use mcp_protocol::resources::ResourceContent; // TODO: Use for advanced resource content

/// Simple configuration file resource
#[derive(McpResource, Serialize, Deserialize)]
#[uri = "file://config.json"]
#[name = "Application Configuration"]
#[description = "Main application configuration file"]
struct ConfigResource {
    #[content]
    #[content_type = "application/json"]
    pub config_data: String,
}

impl ConfigResource {
    fn new() -> Self {
        let config = serde_json::json!({
            "app_name": "MCP Resource Server",
            "version": "1.0.0",
            "debug": true,
            "features": ["resources", "derive_macros", "json_config"]
        });
        
        Self {
            config_data: serde_json::to_string_pretty(&config).unwrap(),
        }
    }
}

/// System status resource (unit struct)
#[derive(McpResource)]
#[uri = "system://status"]
#[name = "System Status"]
#[description = "Current system status and health information"]
struct SystemStatusResource;

/// User data resource with multiple content fields
#[derive(McpResource, Serialize, Deserialize)]
#[uri = "data://user-profile"]
#[name = "User Profile"]
#[description = "User profile data with multiple content sections"]
struct UserProfileResource {
    #[content]
    #[content_type = "application/json"]
    pub profile_data: String,
    
    #[content]
    #[content_type = "text/plain"]
    pub bio: String,
    
    pub internal_id: u64, // This won't be included as content
}

impl UserProfileResource {
    fn new() -> Self {
        let profile = serde_json::json!({
            "username": "demo_user",
            "email": "demo@example.com",
            "created_at": "2023-01-01T00:00:00Z",
            "preferences": {
                "theme": "dark",
                "notifications": true
            }
        });
        
        Self {
            profile_data: serde_json::to_string_pretty(&profile).unwrap(),
            bio: "This is a demo user account created for testing the MCP resource server functionality.".to_string(),
            internal_id: 12345,
        }
    }
}

/// Log file resource (tuple struct)
#[derive(McpResource)]
#[uri = "file://app.log"]
#[name = "Application Log"]
#[description = "Current application log entries"]
struct LogFileResource(String);

impl LogFileResource {
    fn new() -> Self {
        let log_content = vec![
            "2024-01-01 10:00:00 INFO  Server starting up",
            "2024-01-01 10:00:01 INFO  Configuration loaded",
            "2024-01-01 10:00:02 INFO  MCP resources initialized",
            "2024-01-01 10:00:03 INFO  Server ready to accept connections",
            "2024-01-01 10:00:10 DEBUG Resource accessed: config.json",
            "2024-01-01 10:00:15 DEBUG Resource accessed: user-profile",
        ].join("\n");
        
        Self(log_content)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("Starting MCP Resource Server with #[derive(McpResource)] examples");

    // Create resource instances
    let config_resource = ConfigResource::new();
    let system_status = SystemStatusResource;
    let user_profile = UserProfileResource::new();
    let log_file = LogFileResource::new();

    let server = McpServer::builder()
        .name("resource-server")
        .version("1.0.0")
        .title("Resource Server Example")
        .instructions("This server demonstrates the #[derive(McpResource)] macro with various resource types.")
        // TODO: Add resource registration when framework supports it
        .bind_address("127.0.0.1:8007".parse()?)
        .build()?;

    println!("Resource server running at: http://127.0.0.1:8007/mcp");
    println!("\nResource examples implemented:");
    println!("  1. ConfigResource - JSON configuration with #[content] field");
    println!("  2. SystemStatusResource - Unit struct with default implementation");
    println!("  3. UserProfileResource - Multiple content fields with different types");
    println!("  4. LogFileResource - Tuple struct with single content field");
    
    println!("\nResource URIs:");
    println!("  - {}", config_resource.uri());
    println!("  - {}", system_status.uri());
    println!("  - {}", user_profile.uri());
    println!("  - {}", log_file.uri());

    // Demonstrate resource functionality
    println!("\nTesting resource read functionality:");
    
    println!("\n1. Config Resource:");
    match config_resource.read().await {
        Ok(content) => {
            for (i, item) in content.iter().enumerate() {
                println!("   Content {}: {:?}", i + 1, item);
            }
        }
        Err(e) => println!("   Error: {}", e),
    }
    
    println!("\n2. System Status Resource:");
    match system_status.read().await {
        Ok(content) => {
            for (i, item) in content.iter().enumerate() {
                println!("   Content {}: {:?}", i + 1, item);
            }
        }
        Err(e) => println!("   Error: {}", e),
    }

    println!("\n3. User Profile Resource:");
    match user_profile.read().await {
        Ok(content) => {
            for (i, item) in content.iter().enumerate() {
                println!("   Content {}: {:?}", i + 1, item);
            }
        }
        Err(e) => println!("   Error: {}", e),
    }

    println!("\n4. Log File Resource:");
    match log_file.read().await {
        Ok(content) => {
            for (i, item) in content.iter().enumerate() {
                println!("   Content {}: {:?}", i + 1, item);
            }
        }
        Err(e) => println!("   Error: {}", e),
    }

    server.run().await?;
    Ok(())
}