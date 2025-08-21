//! # Version Negotiation Server Example
//!
//! This example demonstrates MCP protocol version negotiation between client and server.
//! The server supports multiple protocol versions and automatically negotiates the best
//! compatible version during the initialize handshake.

use std::collections::HashMap;
use async_trait::async_trait;
use mcp_server::{McpServer, McpTool};
use mcp_protocol::{ToolSchema, ToolResult, version::McpVersion, schema::JsonSchema, McpError, McpResult};
use serde_json::Value;
use tracing::info;

/// Simple version info tool that shows negotiated protocol version
struct VersionInfoTool;

#[async_trait]
impl McpTool for VersionInfoTool {
    fn name(&self) -> &str {
        "version_info"
    }

    fn description(&self) -> &str {
        "Get information about the negotiated MCP protocol version and capabilities"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
    }

    async fn call(
        &self,
        _args: Value,
        session: Option<mcp_server::SessionContext>,
    ) -> McpResult<Vec<ToolResult>> {
        let mut results = vec![];

        if let Some(ctx) = session {
            // Try to get version information from session state
            if let Some(version_info) = (ctx.get_state)("mcp_version") {
                let version_str = version_info.as_str().unwrap_or("unknown");
                results.push(ToolResult::text(format!(
                    "Protocol Version: {}\nSession ID: {}",
                    version_str, ctx.session_id
                )));

                // Add detailed capability info based on version
                if let Some(version) = McpVersion::from_str(version_str) {
                    let features = version.supported_features();
                    results.push(ToolResult::text(format!(
                        "Supported Features: {}",
                        features.join(", ")
                    )));

                    results.push(ToolResult::text(format!(
                        "Version Capabilities:\n\
                        - Streamable HTTP: {}\n\
                        - Meta Fields: {}\n\
                        - Progress & Cursor: {}\n\
                        - Elicitation: {}",
                        version.supports_streamable_http(),
                        version.supports_meta_fields(),
                        version.supports_progress_and_cursor(),
                        version.supports_elicitation()
                    )));
                }
            } else {
                results.push(ToolResult::text(
                    "Version information not available in session".to_string()
                ));
            }
        } else {
            results.push(ToolResult::text(
                "No session context available".to_string()
            ));
        }

        Ok(results)
    }
}

/// Test version negotiation with different client versions
struct VersionTestTool;

#[async_trait]
impl McpTool for VersionTestTool {
    fn name(&self) -> &str {
        "test_version_negotiation"
    }

    fn description(&self) -> &str {
        "Test version negotiation by simulating different client version requests"
    }

    fn input_schema(&self) -> ToolSchema {
        let mut properties = HashMap::new();
        properties.insert("client_version".to_string(), JsonSchema::string());
        
        ToolSchema::object()
            .with_properties(properties)
            .with_required(vec!["client_version".to_string()])
    }

    async fn call(
        &self,
        args: Value,
        _session: Option<mcp_server::SessionContext>,
    ) -> McpResult<Vec<ToolResult>> {
        let client_version = args.get("client_version")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("client_version"))?;

        // Simulate version negotiation logic
        let negotiation_result = match McpVersion::from_str(client_version) {
            Some(requested) => {
                let supported_versions = vec![
                    McpVersion::V2024_11_05,
                    McpVersion::V2025_03_26,
                    McpVersion::V2025_06_18,
                ];

                if supported_versions.contains(&requested) {
                    format!("✅ Version {} accepted as requested", requested)
                } else {
                    format!("❌ Version {} not supported", client_version)
                }
            }
            None => format!("❌ Invalid version format: {}", client_version),
        };

        let results = vec![
            ToolResult::text(format!(
                "Version Negotiation Test\n\
                Client Requested: {}\n\
                Server Response: {}\n\
                Server Supports: 2024-11-05, 2025-03-26, 2025-06-18",
                client_version,
                negotiation_result
            )),
        ];

        Ok(results)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🔄 Starting MCP Version Negotiation Server");

    let server = McpServer::builder()
        .name("version-negotiation-server")
        .version("1.0.0")
        .title("MCP Protocol Version Negotiation Example")
        .instructions("This server demonstrates automatic MCP protocol version negotiation. The server supports versions 2024-11-05, 2025-03-26, and 2025-06-18, and will negotiate the best compatible version during initialization.")
        .tool(VersionInfoTool)
        .tool(VersionTestTool)
        .bind_address("127.0.0.1:8049".parse()?)
        .build()?;

    info!("🚀 Version Negotiation server running at: http://127.0.0.1:8049/mcp");
    info!("");
    info!("📋 Features demonstrated:");
    info!("  ✅ Automatic protocol version negotiation");
    info!("  ✅ Backward compatibility with older versions");
    info!("  ✅ Session-aware version tracking");
    info!("  ✅ Capability adjustment based on negotiated version");
    info!("");
    info!("🔧 Available tools:");
    info!("  📊 version_info - Get negotiated version and capabilities");
    info!("  🧪 test_version_negotiation - Test negotiation with different versions");
    info!("");
    info!("🔄 Supported protocol versions:");
    info!("  • 2024-11-05 - Base protocol");
    info!("  • 2025-03-26 - Added streamable HTTP/SSE");
    info!("  • 2025-06-18 - Added _meta fields, progress tokens, cursors");
    info!("");
    info!("💡 Test version negotiation:");
    info!("  curl -X POST http://127.0.0.1:8049/mcp \\");
    info!("    -H 'Content-Type: application/json' \\");
    info!("    -d '{{\"method\": \"initialize\", \"params\": {{\"protocol_version\": \"2025-06-18\", \"capabilities\": {{}}, \"client_info\": {{\"name\": \"test-client\", \"version\": \"1.0.0\"}}}}}}'");

    server.run().await?;
    Ok(())
}