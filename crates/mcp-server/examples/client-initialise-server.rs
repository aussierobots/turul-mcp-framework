//! # MCP Initialize Test Server
//!
//! A simple MCP server for testing session management and initialize lifecycle.
//! This server implements proper MCP session creation where:
//! - Server generates session IDs (not client)
//! - Session IDs are returned in Mcp-Session-Id headers
//! - Sessions persist for subsequent requests
//!
//! ## Usage
//! ```bash
//! # Start server on default port (8000)
//! cargo run --example client-initialise-server
//! ```
//!
//! ## Test with Client
//! ```bash
//! # In another terminal:
//! cargo run --example client-initialise-report -- --url http://127.0.0.1:8000/mcp
//! ```

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use tracing::info;

use mcp_protocol::tools::ToolResult;
use mcp_protocol::{CallToolResult, McpResult};
use mcp_protocol_2025_06_18::tools::{
    HasAnnotations, HasBaseMetadata, HasDescription, HasInputSchema, HasOutputSchema, HasToolMeta,
    ToolAnnotations, ToolSchema,
};
use mcp_protocol::JsonSchema;
use mcp_server::{McpServer, McpTool, SessionContext};

/// EchoSSE Tool for testing server-side logging and SSE streaming
pub struct EchoSseTool {
    input_schema: ToolSchema,
}

impl EchoSseTool {
    pub fn new() -> Self {
        let input_schema = ToolSchema::object()
            .with_properties(HashMap::from([(
                "text".to_string(),
                JsonSchema::string().with_description("Text to echo back"),
            )]))
            .with_required(vec!["text".to_string()]);
        Self { input_schema }
    }
}

// Implement all the fine-grained traits for ToolDefinition
impl HasBaseMetadata for EchoSseTool {
    fn name(&self) -> &str {
        "echo_sse"
    }
    fn title(&self) -> Option<&str> {
        Some("Echo SSE")
    }
}

impl HasDescription for EchoSseTool {
    fn description(&self) -> Option<&str> {
        Some("Echoes text back via POST response and streams it via SSE. Server logs all calls.")
    }
}

impl HasInputSchema for EchoSseTool {
    fn input_schema(&self) -> &ToolSchema {
        &self.input_schema
    }
}

impl HasOutputSchema for EchoSseTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}

impl HasAnnotations for EchoSseTool {
    fn annotations(&self) -> Option<&ToolAnnotations> {
        None
    }
}

impl HasToolMeta for EchoSseTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

// ToolDefinition is automatically implemented via blanket impl!

#[async_trait]
impl McpTool for EchoSseTool {
    async fn call(
        &self,
        args: Value,
        session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        // Extract text parameter
        let text = args
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| mcp_protocol::McpError::missing_param("text"))?;

        // Log the call on the server side
        info!("🔊 echo_sse called with text: '{}'", text);
        info!(
            "📡 Session ID: {}",
            session
                .as_ref()
                .map(|s| s.session_id.as_str())
                .unwrap_or("no-session")
        );

        // Create response for POST
        let response_text = format!("Echo: {}", text);
        let post_result = CallToolResult::success(vec![ToolResult::text(response_text.clone())]);

        // Send via SSE using proper MCP notifications if session is available
        if let Some(session_ctx) = &session {
            info!(
                "📤 Sending echo response via proper MCP notification to session: {}",
                session_ctx.session_id
            );

            // Send proper MCP notifications/message notification
            session_ctx.notify_log("info", format!("Echo tool response: {}", response_text));

            // Also send a progress notification to demonstrate proper MCP format
            session_ctx.notify_progress("echo-operation", 100);

            info!(
                "✅ Echo response sent via both POST and proper MCP notifications (message + progress)"
            );
        } else {
            info!("⚠️  No session context available - MCP notifications not sent");
        }

        Ok(post_result)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("🚀 Starting MCP Initialize Test Server");
    info!("   • Server creates and manages session IDs");
    info!("   • Session IDs returned via Mcp-Session-Id header");

    // Parse command line arguments
    let port = std::env::args()
        .nth(1)
        .and_then(|arg| {
            if arg == "--port" {
                std::env::args().nth(2).and_then(|p| p.parse().ok())
            } else {
                None
            }
        })
        .unwrap_or(8000);

    let bind_address: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse()?;
    info!("   • Binding to: http://{}/mcp", bind_address);

    // Build server using builder pattern
    let server = McpServer::builder()
        .name("client-initialise-server")
        .version("1.0.0")
        .title("MCP Initialize Test Server")
        .bind_address(bind_address)
        .tool(EchoSseTool::new())
        .build()?;

    info!("✅ Server configured with proper session management");
    info!("📡 Ready to accept initialize requests");
    info!("");
    info!("🧪 Test with client:");
    info!(
        "   cargo run --example client-initialise-report -- --url http://127.0.0.1:{}/mcp",
        port
    );
    info!("");
    info!("📋 Manual curl test:");
    info!("   curl -X POST http://127.0.0.1:{}/mcp \\", port);
    info!("     -H \"Content-Type: application/json\" \\");
    info!(
        "     -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{{\"protocolVersion\":\"2025-06-18\",\"capabilities\":{{}},\"clientInfo\":{{\"name\":\"test\",\"version\":\"1.0\"}}}}}}' \\"
    );
    info!("     -i");
    info!("");

    // Start the server
    server.run().await?;

    Ok(())
}
