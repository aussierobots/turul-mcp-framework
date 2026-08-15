//! Thin wrapper over `turul-mcp-client` for talking to `lambda-mcp-server`.
//!
//! Everything reported here comes from the framework client. `turul-mcp-client`
//! does not retain the 2025-11-25 `initialize` result — `discovered_server()`
//! and `server_capabilities()` are populated from `server/discover` on 2026
//! connections only, and the negotiated `Mcp-Session-Id` is private to the
//! transport. So this wrapper reports the negotiated wire version and nothing
//! more; it must not synthesise capabilities or a session id it cannot observe.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::{debug, info};

use turul_mcp_client::{
    McpClient as FrameworkClient,
    config::ClientConfig,
    transport::{BoxedTransport, HttpTransport},
};

pub use turul_mcp_client::{CallToolResult, Tool};

/// What the client can actually establish about the peer after connecting.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionInfo {
    /// Wire spec negotiated for this connection, as reported by the framework.
    pub negotiated_version: Option<String>,
    /// Endpoint the transport is posting to.
    pub endpoint: String,
    /// Server capabilities when the peer supplied them. Always `None` on a
    /// 2025-11-25 connection.
    pub capabilities: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListToolsResult {
    pub tools: Vec<Tool>,
}

#[derive(Debug, Clone)]
pub struct McpClientConfig {
    /// Base URL of the MCP server; `/mcp` is appended when absent.
    pub base_url: String,
}

pub struct McpClient {
    framework_client: FrameworkClient,
    endpoint: String,
}

impl McpClient {
    pub async fn new(config: McpClientConfig) -> Result<Self> {
        let endpoint = if config.base_url.ends_with("/mcp") {
            config.base_url.clone()
        } else {
            format!("{}/mcp", config.base_url.trim_end_matches('/'))
        };

        let transport: BoxedTransport =
            Box::new(HttpTransport::new(&endpoint).context("Failed to create HTTP transport")?);
        let framework_client = FrameworkClient::new(transport, ClientConfig::default());

        Ok(Self {
            framework_client,
            endpoint,
        })
    }

    /// Connect. On the 2025-11-25 lane the framework performs `initialize` and
    /// `notifications/initialized` internally and holds the session id.
    pub async fn connect(&mut self) -> Result<ConnectionInfo> {
        info!("🚀 Connecting to {}", self.endpoint);

        self.framework_client
            .connect()
            .await
            .context("Framework connection failed")?;

        let info = ConnectionInfo {
            negotiated_version: self
                .framework_client
                .negotiated_version()
                .await
                .map(|v| v.to_string()),
            endpoint: self.endpoint.clone(),
            capabilities: self.framework_client.server_capabilities().await,
        };

        info!("✅ Connected — negotiated {:?}", info.negotiated_version);
        Ok(info)
    }

    pub async fn list_tools(&self) -> Result<ListToolsResult> {
        let tools = self
            .framework_client
            .list_tools()
            .await
            .context("Framework list_tools failed")?;
        debug!("Found {} tools", tools.len());
        Ok(ListToolsResult { tools })
    }

    pub async fn call_tool(&self, name: &str, arguments: Option<Value>) -> Result<CallToolResult> {
        let args = arguments.unwrap_or(json!({}));
        self.framework_client
            .call_tool(name, args)
            .await
            .context("Framework call_tool failed")
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}
