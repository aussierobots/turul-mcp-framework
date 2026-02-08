//! # Simple Resources Demo - New Trait Architecture
//!
//! This demonstrates MCP resources using the new fine-grained trait architecture.
//! Shows both derive macro and manual trait implementation approaches.

use async_trait::async_trait;
use turul_mcp_derive::McpResource;
use turul_mcp_server::{McpServer, McpResult, McpResource};
use turul_mcp_protocol::resources::ResourceContent;
use serde_json::json;
use tracing::info;

// =============================================================================
// APPROACH 1: Using Derive Macro (Recommended)
// =============================================================================

#[derive(McpResource)]
#[uri = "docs://project/readme"]
#[name = "Project README"]
#[description = "Main project documentation and getting started guide"]
struct ProjectReadme;

impl ProjectReadme {
    async fn read_impl(&self, _params: Option<serde_json::Value>) -> McpResult<Vec<ResourceContent>> {
        Ok(vec![ResourceContent::text(format!(
            "# Project Documentation\n\nWelcome to our project!\n\n## Getting Started\n\n1. Install dependencies\n2. Run the server\n3. Connect your MCP client\n\n## Features\n\n- ✅ Zero-configuration setup\n- ✅ Fine-grained trait architecture  \n- ✅ MCP 2025-11-25 specification compliant\n- ✅ Real-time SSE notifications\n\nLast updated: {}",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ))])
    }
}

#[derive(McpResource)]
#[uri = "config://app/settings"]
#[name = "App Configuration"]
#[description = "Current application configuration and settings"]
struct AppConfig;

impl AppConfig {
    async fn read_impl(&self, _params: Option<serde_json::Value>) -> McpResult<Vec<ResourceContent>> {
        let config = json!({
            "server": {
                "name": "simple-resources-demo",
                "version": "1.0.0",
                "port": 8080
            },
            "features": {
                "sse_enabled": true,
                "session_storage": "InMemory",
                "protocol_version": "2025-06-18"
            },
            "resources": {
                "total_count": 2,
                "types": ["documentation", "configuration"]
            }
        });

        Ok(vec![ResourceContent::blob(
            serde_json::to_string_pretty(&config).unwrap(),
            "application/json".to_string()
        )])
    }
}

// =============================================================================
// MAIN SERVER SETUP
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 Simple Resources Demo - New Trait Architecture");
    info!("   • Using derive macros with new fine-grained traits");
    info!("   • MCP 2025-11-25 compliant resource definitions");
    info!("   • Zero-configuration resource setup");

    let server = McpServer::builder()
        .name("simple-resources-demo")
        .version("1.0.0")
        .title("Simple Resources Demo")
        .instructions("Demonstrating MCP resources with new trait architecture")
        // Add resources using derive macro approach
        .resource(ProjectReadme)
        .resource(AppConfig)
        .bind_address("127.0.0.1:8088".parse()?)
        .sse(true)
        .build()?;

    info!("✨ Resources Available:");
    info!("   • docs://project/readme - Project documentation");
    info!("   • config://app/settings - Application configuration");
    info!("🎯 Server running at: http://127.0.0.1:8088/mcp");

    server.run().await?;
    Ok(())
}