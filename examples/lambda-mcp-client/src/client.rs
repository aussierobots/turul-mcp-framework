//! MCP Client Implementation
//!
//! This module provides a comprehensive client for communicating with MCP servers
//! following the MCP 2025-06-18 Streamable HTTP specification.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::{Client, Response};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Configuration for MCP client
#[derive(Debug, Clone)]
pub struct McpClientConfig {
    /// Base URL of the MCP server
    pub base_url: String,
    /// Request timeout
    pub timeout: Duration,
    /// User agent string
    pub user_agent: String,
}

/// MCP client for communicating with servers
#[derive(Debug)]
pub struct McpClient {
    /// HTTP client
    client: Client,
    /// Client configuration
    config: McpClientConfig,
    /// Session ID for this client
    session_id: String,
    /// Server capabilities from initialization
    server_capabilities: Option<Value>,
    /// Protocol version negotiated
    protocol_version: Option<String>,
    /// Request ID counter for JSON-RPC requests
    next_request_id: std::sync::atomic::AtomicI64,
}

impl McpClient {
    /// Create a new MCP client
    pub fn new(config: McpClientConfig, session_id: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(config.timeout)
            .user_agent(&config.user_agent)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            config,
            session_id,
            server_capabilities: None,
            protocol_version: None,
            next_request_id: std::sync::atomic::AtomicI64::new(1),
        })
    }

    /// Get next request ID for JSON-RPC requests
    fn next_id(&self) -> i64 {
        self.next_request_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// Initialize the MCP session
    pub async fn initialize(&mut self) -> Result<Value> {
        info!("Initializing MCP session (will get session ID from server)");
        
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "tools": {},
                    "resources": {},
                    "notifications": {}
                },
                "clientInfo": {
                    "name": "lambda-mcp-client",
                    "version": "0.1.0"
                }
            }
        });

        // For initialize, we don't send session ID and extract it from response
        let response = self.send_initialize_request(request).await?;
        
        // Store server capabilities
        if let Some(result) = response.get("result") {
            self.server_capabilities = result.get("capabilities").cloned();
            self.protocol_version = result.get("protocolVersion")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }

        // Send initialized notification
        let initialized_request = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });

        let _initialized_response = self.send_request(initialized_request).await?;
        
        Ok(response)
    }

    /// List available tools
    pub async fn list_tools(&self) -> Result<Value> {
        debug!("Listing tools for session: {}", self.session_id);
        
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "tools/list"
        });

        self.send_request(request).await
    }

    /// Call a tool
    pub async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<Value> {
        debug!("Calling tool '{}' for session: {}", tool_name, self.session_id);
        
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            }
        });

        self.send_request(request).await
    }

    /// List available resources
    pub async fn list_resources(&self) -> Result<Value> {
        debug!("Listing resources for session: {}", self.session_id);
        
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "resources/list"
        });

        self.send_request(request).await
    }

    /// Read a resource
    pub async fn read_resource(&self, uri: &str) -> Result<Value> {
        debug!("Reading resource '{}' for session: {}", uri, self.session_id);
        
        let request = json!({
            "jsonrpc": "2.0",
            "id": Uuid::new_v4().to_string(),
            "method": "resources/read",
            "params": {
                "uri": uri
            }
        });

        self.send_request(request).await
    }

    /// List available prompts
    pub async fn list_prompts(&self) -> Result<Value> {
        debug!("Listing prompts for session: {}", self.session_id);
        
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "prompts/list"
        });

        self.send_request(request).await
    }

    /// Get a prompt
    pub async fn get_prompt(&self, name: &str, arguments: Option<Value>) -> Result<Value> {
        debug!("Getting prompt '{}' for session: {}", name, self.session_id);
        
        let mut params = json!({
            "name": name
        });

        if let Some(args) = arguments {
            params["arguments"] = args;
        }

        let request = json!({
            "jsonrpc": "2.0",
            "id": Uuid::new_v4().to_string(),
            "method": "prompts/get",
            "params": params
        });

        self.send_request(request).await
    }

    /// Set logging level
    pub async fn set_logging_level(&self, level: &str) -> Result<Value> {
        debug!("Setting logging level to '{}' for session: {}", level, self.session_id);
        
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "logging/setLevel",
            "params": {
                "level": level
            }
        });

        self.send_request(request).await
    }

    /// Send a ping request
    pub async fn ping(&self) -> Result<Value> {
        debug!("Sending ping for session: {}", self.session_id);
        
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "ping"
        });

        self.send_request(request).await
    }

    /// Get session information (if available)
    pub async fn get_session_info(&self) -> Result<Value> {
        self.call_tool("session_info", json!({})).await
    }

    /// Get lambda diagnostics (if available)
    pub async fn get_lambda_diagnostics(&self) -> Result<Value> {
        self.call_tool("lambda_diagnostics", json!({})).await
    }

    /// Test all available tools
    pub async fn test_all_tools(&self) -> Result<HashMap<String, Result<Value, String>>> {
        let tools_response = self.list_tools().await?;
        let mut results = HashMap::new();

        if let Some(tools) = tools_response.get("result")
            .and_then(|r| r.get("tools"))
            .and_then(|t| t.as_array()) {
            
            for tool in tools {
                if let Some(name) = tool.get("name").and_then(|n| n.as_str()) {
                    debug!("Testing tool: {}", name);
                    
                    let test_args = self.get_test_arguments_for_tool(name, tool);
                    
                    match self.call_tool(name, test_args).await {
                        Ok(result) => {
                            results.insert(name.to_string(), Ok(result));
                        }
                        Err(e) => {
                            results.insert(name.to_string(), Err(e.to_string()));
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    /// Get server capabilities
    pub fn server_capabilities(&self) -> Option<&Value> {
        self.server_capabilities.as_ref()
    }

    /// Get negotiated protocol version
    pub fn protocol_version(&self) -> Option<&str> {
        self.protocol_version.as_deref()
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Send a request to the MCP server
    async fn send_request(&self, request: Value) -> Result<Value> {
        debug!("Sending MCP request: {}", request);

        let response = self.client
            .post(&format!("{}/mcp", self.config.base_url))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header("Mcp-Session-Id", &self.session_id)
            .json(&request)
            .send()
            .await
            .context("Failed to send HTTP request")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "HTTP error: {} - {}", 
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        let response_text = response.text().await
            .context("Failed to read response text")?;

        debug!("Received MCP response: {}", response_text);

        let response_json: Value = serde_json::from_str(&response_text)
            .context("Failed to parse JSON response")?;

        // Check for JSON-RPC errors
        if let Some(error) = response_json.get("error") {
            return Err(anyhow::anyhow!(
                "MCP error: {}",
                serde_json::to_string_pretty(error)?
            ));
        }

        Ok(response_json)
    }

    /// Send initialize request without session ID and extract server-generated session ID
    async fn send_initialize_request(&mut self, request: Value) -> Result<Value> {
        debug!("Sending MCP initialize request (no session ID): {}", request);

        let response = self.client
            .post(&format!("{}/mcp", self.config.base_url))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            // Don't send Mcp-Session-Id header for initialize request
            .json(&request)
            .send()
            .await
            .context("Failed to send HTTP request")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "HTTP error: {} - {}", 
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        // Extract server-generated session ID from response headers
        if let Some(session_id_header) = response.headers().get("mcp-session-id") {
            if let Ok(session_id) = session_id_header.to_str() {
                info!("Server provided session ID: {}", session_id);
                self.session_id = session_id.to_string();
            } else {
                warn!("Server provided invalid session ID header");
            }
        } else {
            warn!("Server did not provide session ID in response headers");
        }

        let response_text = response.text().await
            .context("Failed to read response text")?;

        debug!("Received MCP response: {}", response_text);

        let response_json: Value = serde_json::from_str(&response_text)
            .context("Failed to parse JSON response")?;

        // Check for JSON-RPC errors
        if let Some(error) = response_json.get("error") {
            return Err(anyhow::anyhow!(
                "MCP error: {}",
                serde_json::to_string_pretty(error)?
            ));
        }

        Ok(response_json)
    }

    /// Get appropriate test arguments for a tool
    fn get_test_arguments_for_tool(&self, tool_name: &str, _tool_def: &Value) -> Value {
        match tool_name {
            "lambda_diagnostics" => json!({
                "include_metrics": true,
                "include_environment": true,
                "include_aws_info": false
            }),
            "session_info" => json!({
                "include_capabilities": true,
                "include_statistics": true
            }),
            "aws_real_time_monitor" => json!({
                "resource_type": "Lambda",
                "region": "us-east-1"
            }),
            "list_active_sessions" => json!({}),
            // "publish_test_event" removed - incompatible with clean SNS → Lambda direct triggers + tokio broadcast architecture
            _ => json!({})
        }
    }
}

/// SSE (Server-Sent Events) client for streaming responses
#[derive(Debug)]
pub struct McpSseClient {
    client: Client,
    config: McpClientConfig,
    session_id: String,
}

impl McpSseClient {
    /// Create a new SSE client
    pub fn new(config: McpClientConfig, session_id: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(config.timeout)
            .user_agent(&config.user_agent)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            config,
            session_id,
        })
    }

    /// Subscribe to SSE events from the server
    pub async fn subscribe_to_events(&self) -> Result<impl futures::Stream<Item = Result<String>>> {
        use futures::stream::StreamExt;
        use tokio_stream::wrappers::LinesStream;

        let response = self.client
            .get(&format!("{}/mcp", self.config.base_url))
            .header("Accept", "application/json, text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Mcp-Session-Id", &self.session_id)
            .send()
            .await
            .context("Failed to start SSE connection")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "SSE connection failed: {}", 
                response.status()
            ));
        }

        let stream = response
            .bytes_stream()
            .map(|result| {
                result
                    .map_err(|e| anyhow::anyhow!("Stream error: {}", e))
                    .and_then(|bytes| {
                        String::from_utf8(bytes.to_vec())
                            .map_err(|e| anyhow::anyhow!("UTF-8 error: {}", e))
                    })
            });

        Ok(stream)
    }
}