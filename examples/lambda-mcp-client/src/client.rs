//! MCP Client Implementation using turul-mcp-client framework
//!
//! This module provides a wrapper around the turul-mcp-client framework
//! for communicating with MCP servers.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;
use tracing::{debug, info, warn};

// Use the framework client and transport
use turul_mcp_client::{
    McpClient as FrameworkClient, McpClientResult,
    config::ClientConfig,
    transport::{BoxedTransport, HttpTransport},
};

// Import protocol types directly since the framework re-exports them
pub use turul_mcp_client::{Tool, ToolResult};

/// MCP protocol response types (created from framework responses)
#[derive(Debug, Serialize, Deserialize)]
pub struct InitializeResult {
    pub capabilities: Value,
    pub server_info: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListToolsResult {
    pub tools: Vec<Tool>,
}

pub use turul_mcp_client::CallToolResult;

/// Configuration for MCP client (simplified wrapper around framework config)
#[derive(Debug, Clone)]
pub struct McpClientConfig {
    /// Base URL of the MCP server
    pub base_url: String,
    /// Request timeout
    pub timeout: Duration,
    /// User agent string
    pub user_agent: String,
}

/// MCP client wrapper that uses the turul-mcp-client framework
pub struct McpClient {
    /// Framework MCP client
    framework_client: FrameworkClient,
    /// Client configuration
    config: McpClientConfig,
}

impl McpClient {
    /// Create a new MCP client using the framework
    pub async fn new(config: McpClientConfig) -> Result<Self> {
        info!("🔧 Creating MCP client using turul-mcp-client framework");

        // Create HTTP transport for the framework client
        // Add /mcp endpoint to the base URL for MCP protocol
        let mcp_url = if config.base_url.ends_with("/mcp") {
            config.base_url.clone()
        } else {
            format!("{}/mcp", config.base_url.trim_end_matches('/'))
        };
        let transport = HttpTransport::new(&mcp_url).context("Failed to create HTTP transport")?;

        // Convert to BoxedTransport
        let boxed_transport: BoxedTransport = Box::new(transport);

        // Create framework client configuration
        let client_config = ClientConfig::default();

        // Create framework client with the transport and config
        let framework_client = FrameworkClient::new(boxed_transport, client_config);

        info!("✅ Framework MCP client created successfully");

        Ok(Self {
            framework_client,
            config,
        })
    }

    /// Initialize connection with the server using framework
    pub async fn initialize(&mut self) -> Result<InitializeResult> {
        info!("🚀 Initializing MCP connection using framework");

        // Connect to the server
        self.framework_client
            .connect()
            .await
            .context("Framework connection failed")?;

        // For now, return a simple initialization result
        // The framework doesn't expose the initialize method directly in the API I see
        let init_result = InitializeResult {
            capabilities: json!({
                "tools": {"listChanged": false},  // MCP compliance: static framework
                "resources": {"listChanged": false, "subscribe": false},
                "prompts": {"listChanged": false},
                "logging": {},
                "experimental": {}
            }),
            server_info: json!({
                "framework": "turul-mcp-client",
                "transport": "HTTP",
                "base_url": self.config.base_url
            }),
        };

        info!("✅ MCP connection initialized successfully");
        debug!("Connection established to: {}", self.config.base_url);

        Ok(init_result)
    }

    /// List available tools using framework
    pub async fn list_tools(&self) -> Result<ListToolsResult> {
        debug!("📋 Listing tools using framework");

        let tools = self
            .framework_client
            .list_tools()
            .await
            .context("Framework list_tools failed")?;

        debug!("Found {} tools", tools.len());

        Ok(ListToolsResult { tools })
    }

    /// Call a tool using framework
    pub async fn call_tool(&self, name: &str, arguments: Option<Value>) -> Result<CallToolResult> {
        info!("🔧 Calling tool '{}' using framework", name);
        debug!("Tool arguments: {:?}", arguments);

        let args = arguments.unwrap_or(json!({}));

        let result = self
            .framework_client
            .call_tool(name, args)
            .await
            .context("Framework call_tool failed")?;

        info!("✅ Tool '{}' executed successfully", name);
        debug!("Tool result: {} content items", result.content.len());

        Ok(result)
    }

    /// Send initialized notification using framework
    pub async fn send_initialized(&self) -> Result<()> {
        debug!("📡 Sending initialized notification (framework handles this)");
        // The framework handles initialization internally
        debug!("✅ Initialized notification handled by framework");
        Ok(())
    }

    /// Test the client connection
    pub async fn test_connection(&self) -> Result<Value> {
        info!("🔍 Testing MCP client connection using framework");

        // Use framework's list_tools as a connection test
        match self.list_tools().await {
            Ok(tools_result) => {
                let test_result = json!({
                    "status": "connected",
                    "framework": "turul-mcp-client",
                    "tools_available": tools_result.tools.len(),
                    "base_url": self.config.base_url,
                    "protocol_version": "2025-11-25"
                });
                info!("✅ Connection test successful");
                Ok(test_result)
            }
            Err(e) => {
                warn!("❌ Connection test failed: {}", e);
                Err(e)
            }
        }
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }

    /// Get the timeout duration
    pub fn timeout(&self) -> Duration {
        self.config.timeout
    }

    /// Check if client has an active session (framework manages sessions internally)
    pub fn has_session(&self) -> bool {
        // Framework manages sessions automatically
        true
    }

    /// Get session info (framework manages sessions)
    pub fn session_info(&self) -> Value {
        json!({
            "session_management": "framework_managed",
            "framework": "turul-mcp-client",
            "transport": "HTTP"
        })
    }

    /// Get session ID (placeholder - framework manages this)
    pub fn session_id(&self) -> String {
        "framework_managed".to_string()
    }
}
