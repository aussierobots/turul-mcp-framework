//! # Function Resource Server Example
//!
//! This example demonstrates using .resource_fn() to register function-created resources.
//! It shows both static and template resources using constructor functions.

use async_trait::async_trait;
use serde_json::{Value, json};
use std::collections::HashMap;
use turul_mcp_protocol::resources::{
    HasResourceAnnotations, HasResourceDescription, HasResourceMeta, HasResourceMetadata,
    HasResourceMimeType, HasResourceSize, HasResourceUri, ResourceContent,
};
use turul_mcp_server::{McpResource, McpResult, McpServer, SessionContext};

// Static configuration resource
struct ConfigResource;

impl HasResourceMetadata for ConfigResource {
    fn name(&self) -> &str {
        "config"
    }
}

impl HasResourceDescription for ConfigResource {
    fn description(&self) -> Option<&str> {
        Some("Application configuration file")
    }
}

impl HasResourceUri for ConfigResource {
    fn uri(&self) -> &str {
        "file:///config.json"
    }
}

impl HasResourceMimeType for ConfigResource {
    fn mime_type(&self) -> Option<&str> {
        Some("application/json")
    }
}

impl HasResourceSize for ConfigResource {
    fn size(&self) -> Option<u64> {
        None
    }
}

impl HasResourceAnnotations for ConfigResource {
    fn annotations(&self) -> Option<&turul_mcp_protocol::meta::Annotations> {
        None
    }
}

impl HasResourceMeta for ConfigResource {
    fn resource_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

#[async_trait]
impl McpResource for ConfigResource {
    async fn read(&self, _params: Option<Value>, _session: Option<&SessionContext>) -> McpResult<Vec<ResourceContent>> {
        let config = json!({
            "app_name": "Function Resource Server",
            "version": "1.0.0",
            "debug": true,
            "features": ["function_resources", "constructor_functions", "auto_detection"]
        });

        Ok(vec![ResourceContent::blob(
            "file:///config.json",
            serde_json::to_string_pretty(&config).unwrap(),
            "application/json".to_string(),
        )])
    }
}

// Constructor function for config resource
fn create_config_resource() -> ConfigResource {
    ConfigResource
}

// Template resource for user profiles
struct UserProfileResource;

impl HasResourceMetadata for UserProfileResource {
    fn name(&self) -> &str {
        "user_profile"
    }
}

impl HasResourceDescription for UserProfileResource {
    fn description(&self) -> Option<&str> {
        Some("User profile data for a specific user ID")
    }
}

impl HasResourceUri for UserProfileResource {
    fn uri(&self) -> &str {
        "file:///users/{user_id}.json"
    }
}

impl HasResourceMimeType for UserProfileResource {
    fn mime_type(&self) -> Option<&str> {
        Some("application/json")
    }
}

impl HasResourceSize for UserProfileResource {
    fn size(&self) -> Option<u64> {
        None
    }
}

impl HasResourceAnnotations for UserProfileResource {
    fn annotations(&self) -> Option<&turul_mcp_protocol::meta::Annotations> {
        None
    }
}

impl HasResourceMeta for UserProfileResource {
    fn resource_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

#[async_trait]
impl McpResource for UserProfileResource {
    async fn read(&self, params: Option<Value>, _session: Option<&SessionContext>) -> McpResult<Vec<ResourceContent>> {
        // Extract user_id from template variables
        let user_id = if let Some(params) = &params {
            if let Some(template_vars) = params.get("template_variables") {
                if let Some(vars_obj) = template_vars.as_object() {
                    if let Some(user_id_val) = vars_obj.get("user_id") {
                        user_id_val.as_str().unwrap_or("unknown")
                    } else {
                        "unknown"
                    }
                } else {
                    "unknown"
                }
            } else {
                "unknown"
            }
        } else {
            "unknown"
        };

        let profile = json!({
            "user_id": user_id,
            "username": format!("user_{}", user_id),
            "email": format!("user_{}@example.com", user_id),
            "created_at": "2024-01-01T00:00:00Z",
            "profile": {
                "bio": format!("This is user {} - a demo profile", user_id),
                "avatar": format!("https://example.com/avatars/{}.png", user_id),
                "preferences": {
                    "theme": "dark",
                    "notifications": true
                }
            }
        });

        Ok(vec![ResourceContent::blob(
            format!("file:///users/{}.json", user_id),
            serde_json::to_string_pretty(&profile).unwrap(),
            "application/json".to_string(),
        )])
    }
}

// Constructor function for user profile resource
fn create_user_profile_resource() -> UserProfileResource {
    UserProfileResource
}

// System status resource
struct SystemStatusResource;

impl HasResourceMetadata for SystemStatusResource {
    fn name(&self) -> &str {
        "system_status"
    }
}

impl HasResourceDescription for SystemStatusResource {
    fn description(&self) -> Option<&str> {
        Some("Current system health and status information")
    }
}

impl HasResourceUri for SystemStatusResource {
    fn uri(&self) -> &str {
        "system://status"
    }
}

impl HasResourceMimeType for SystemStatusResource {
    fn mime_type(&self) -> Option<&str> {
        Some("application/json")
    }
}

impl HasResourceSize for SystemStatusResource {
    fn size(&self) -> Option<u64> {
        None
    }
}

impl HasResourceAnnotations for SystemStatusResource {
    fn annotations(&self) -> Option<&turul_mcp_protocol::meta::Annotations> {
        None
    }
}

impl HasResourceMeta for SystemStatusResource {
    fn resource_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

#[async_trait]
impl McpResource for SystemStatusResource {
    async fn read(&self, _params: Option<Value>, _session: Option<&SessionContext>) -> McpResult<Vec<ResourceContent>> {
        let status = json!({
            "status": "healthy",
            "uptime": "72h 15m 32s",
            "version": "1.0.0",
            "memory": {
                "used": "256MB",
                "available": "512MB",
                "usage_percent": 50
            },
            "cpu": {
                "usage_percent": 12.5,
                "load_average": [0.1, 0.2, 0.15]
            },
            "network": {
                "active_connections": 42,
                "bytes_sent": "1.2GB",
                "bytes_received": "800MB"
            },
            "services": {
                "mcp_server": "running",
                "database": "running",
                "cache": "running"
            },
            "last_restart": "2024-01-01T10:30:00Z",
            "next_maintenance": "2024-01-08T02:00:00Z"
        });

        Ok(vec![ResourceContent::blob(
            "system://status",
            serde_json::to_string_pretty(&status).unwrap(),
            "application/json".to_string(),
        )])
    }
}

// Constructor function for system status resource
fn create_system_status_resource() -> SystemStatusResource {
    SystemStatusResource
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🎯 Starting Function Resource MCP Server");
    println!("✨ Demonstrating .resource_fn() with constructor functions");

    let server = McpServer::builder()
        .name("function-resource-server")
        .version("1.0.0")
        .title("Function Resource Example Server")
        .instructions("This server demonstrates the .resource_fn() method for registering resources using constructor functions with automatic template detection.")
        .resource_fn(create_config_resource)       // Static resource
        .resource_fn(create_user_profile_resource) // Template: file:///users/{user_id}.json
        .resource_fn(create_system_status_resource) // Static with custom scheme
        .bind_address("127.0.0.1:8008".parse()?)
        .build()?;

    println!("🚀 Function Resource server running at: http://127.0.0.1:8008/mcp");
    println!("📋 Available resources created via constructor functions:");
    println!("  📊 Static Resources:");
    println!("    - config: file:///config.json");
    println!("    - system_status: system://status");
    println!("  🔗 Template Resources:");
    println!("    - user_profile: file:///users/{{user_id}}.json");
    println!("\n✅ resource_fn() pattern successfully implemented:");
    println!("  // Define resource struct with McpResource trait");
    println!("  struct ConfigResource;");
    println!("  impl McpResource for ConfigResource {{ ... }}");
    println!("  ");
    println!("  // Create constructor function");
    println!("  fn create_config_resource() -> ConfigResource {{");
    println!("      ConfigResource");
    println!("  }}");
    println!("  ");
    println!("  // Register with .resource_fn()");
    println!("  let server = McpServer::builder()");
    println!("      .resource_fn(create_config_resource)  // Auto-detects URI type!");
    println!("      .build()?;");
    println!("\n🎨 Features demonstrated:");
    println!("  ✨ Constructor function pattern for resource creation");
    println!("  ✨ Automatic URI template detection ({{user_id}} patterns)");
    println!("  ✨ Template variable extraction from params");
    println!("  ✨ Static resource support (no template variables)");
    println!("  ✨ Custom URI schemes (system://, file://)");
    println!("  ✨ Automatic handler registration via .resource_fn()");
    println!("\n🔧 Template Variable Extraction:");
    println!("  📌 URI: file:///users/{{user_id}}.json");
    println!("  📌 Framework extracts user_id from params.template_variables");
    println!("  📌 Resource implementation reads: params.template_variables.user_id");
    println!("\n💡 Comparison with .resource() method:");
    println!("  📌 .resource(ConfigResource) - Direct instance");
    println!("  📌 .resource_fn(create_config_resource) - Constructor function");
    println!("  📌 Both methods support the same auto-detection features");

    server.run().await?;
    Ok(())
}
