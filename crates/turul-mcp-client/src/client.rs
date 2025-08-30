//! Main MCP client implementation

use serde_json::{Value, json};
use std::sync::Arc;
use tokio::time::timeout;
use tracing::{debug, info, warn};

use crate::config::ClientConfig;
use crate::error::{McpClientError, McpClientResult, SessionError};
use crate::session::{SessionManager, SessionState};
use crate::streaming::StreamHandler;
use crate::transport::{BoxedTransport, TransportFactory};

// Re-export protocol types for convenience
use turul_mcp_protocol::{
    CallToolResult, GetPromptResult, InitializeResult, ListPromptsResult,
    ListResourcesResult, ListToolsResult, Prompt, ReadResourceResult, Resource, Tool,
    ToolResult,
};

/// Main MCP client
pub struct McpClient {
    /// Transport layer
    transport: Arc<tokio::sync::Mutex<BoxedTransport>>,
    /// Session manager
    session: Arc<SessionManager>,
    /// Configuration
    config: ClientConfig,
    /// Stream handler for server events
    stream_handler: Arc<tokio::sync::Mutex<StreamHandler>>,
    /// Request ID counter
    request_counter: Arc<std::sync::atomic::AtomicU64>,
}

impl McpClient {
    /// Create a new MCP client with the given transport
    pub fn new(transport: BoxedTransport, config: ClientConfig) -> Self {
        let session = Arc::new(SessionManager::new(config.clone()));

        Self {
            transport: Arc::new(tokio::sync::Mutex::new(transport)),
            session,
            config,
            stream_handler: Arc::new(tokio::sync::Mutex::new(StreamHandler::new())),
            request_counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Connect to the MCP server
    pub async fn connect(&self) -> McpClientResult<()> {
        info!("Connecting to MCP server");

        // Connect transport
        {
            let mut transport = self.transport.lock().await;
            transport.connect().await?;

            // Start event listener if supported
            if transport.capabilities().server_events {
                let receiver = transport.start_event_listener().await?;
                let mut stream_handler = self.stream_handler.lock().await;
                stream_handler.set_receiver(receiver);
                stream_handler.start().await?;
            }
        }

        // Initialize session
        self.initialize_session().await?;

        info!("Successfully connected to MCP server");
        Ok(())
    }

    /// Disconnect from the MCP server
    pub async fn disconnect(&self) -> McpClientResult<()> {
        info!("Disconnecting from MCP server");

        // Terminate session
        self.session
            .terminate(Some("Client disconnect".to_string()))
            .await;

        // Disconnect transport
        let mut transport = self.transport.lock().await;
        transport.disconnect().await?;

        info!("Disconnected from MCP server");
        Ok(())
    }

    /// Check if client is connected and ready
    pub async fn is_ready(&self) -> bool {
        let transport_connected = {
            let transport = self.transport.lock().await;
            transport.is_connected()
        };

        let session_ready = self.session.is_ready().await;

        transport_connected && session_ready
    }

    /// Get client connection status
    pub async fn connection_status(&self) -> ConnectionStatus {
        let transport_info = {
            let transport = self.transport.lock().await;
            transport.connection_info()
        };

        let session_stats = self.session.statistics().await;

        ConnectionStatus {
            transport_connected: transport_info.connected,
            session_state: session_stats.state,
            transport_type: transport_info.transport_type,
            endpoint: transport_info.endpoint,
            session_id: session_stats.session_id,
            protocol_version: session_stats.protocol_version,
        }
    }

    /// Initialize session with server
    async fn initialize_session(&self) -> McpClientResult<()> {
        info!("Initializing MCP session");

        self.session.mark_initializing().await?;

        let init_request = self.session.create_initialize_request().await;
        let request_json = serde_json::to_value(&init_request).map_err(|e| {
            McpClientError::generic(format!("Failed to serialize initialize request: {}", e))
        })?;

        let json_rpc_request = json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": self.next_request_id(),
            "params": request_json
        });

        // Send initialize request with timeout
        let response = timeout(
            self.config.timeouts.initialization,
            self.send_request_internal(json_rpc_request),
        )
        .await
        .map_err(|_| McpClientError::Timeout)?;

        let response = response?;

        // Parse initialize response
        let init_response: InitializeResult =
            serde_json::from_value(response.get("result").cloned().unwrap_or(Value::Null))
                .map_err(|e| {
                    McpClientError::generic(format!("Failed to parse initialize response: {}", e))
                })?;

        // Validate server capabilities
        self.session
            .validate_server_capabilities(&init_response.capabilities)
            .await?;

        // Complete session initialization
        self.session
            .initialize(
                init_request.capabilities,
                init_response.capabilities,
                init_response.protocol_version,
            )
            .await?;

        // Send initialized notification
        let initialized_notification = json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        });

        self.send_notification_internal(initialized_notification)
            .await?;

        info!("MCP session initialized successfully");
        Ok(())
    }

    /// Generate next request ID
    fn next_request_id(&self) -> String {
        let counter = self
            .request_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        format!("req_{}", counter)
    }

    /// Send request and handle retries
    async fn send_request_internal(&self, request: Value) -> McpClientResult<Value> {
        let mut last_error = None;

        for attempt in 0..self.config.retry.max_attempts {
            if attempt > 0 {
                let delay = self.config.retry.delay_for_attempt(attempt);
                debug!(
                    attempt = attempt,
                    delay_ms = delay.as_millis(),
                    "Retrying request"
                );
                tokio::time::sleep(delay).await;
            }

            match self.send_request_raw(request.clone()).await {
                Ok(response) => {
                    self.session.update_activity().await;
                    return Ok(response);
                }
                Err(e) => {
                    warn!(attempt = attempt, error = %e, "Request failed");

                    if !e.is_retryable() || !self.config.retry.should_retry(attempt + 1) {
                        return Err(e);
                    }

                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| McpClientError::generic("All retry attempts failed")))
    }

    /// Send raw request without retries
    async fn send_request_raw(&self, request: Value) -> McpClientResult<Value> {
        if !self.session.is_ready().await {
            return Err(SessionError::NotInitialized.into());
        }

        let mut transport = self.transport.lock().await;

        let response = timeout(
            self.config.timeouts.request,
            transport.send_request(request),
        )
        .await
        .map_err(|_| McpClientError::Timeout)??;

        // Check for JSON-RPC error
        if let Some(error) = response.get("error") {
            let code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(-1) as i32;
            let message = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");
            let data = error.get("data").cloned();

            return Err(McpClientError::server_error(code, message, data));
        }

        Ok(response)
    }

    /// Send notification
    async fn send_notification_internal(&self, notification: Value) -> McpClientResult<()> {
        let mut transport = self.transport.lock().await;
        transport.send_notification(notification).await?;
        self.session.update_activity().await;
        Ok(())
    }

    /// List available tools
    pub async fn list_tools(&self) -> McpClientResult<Vec<Tool>> {
        debug!("Listing tools");

        let request = json!({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": self.next_request_id(),
            "params": {}
        });

        let response = self.send_request_internal(request).await?;
        let tools_response: ListToolsResult =
            serde_json::from_value(response.get("result").cloned().unwrap_or(Value::Null))?;

        debug!(count = tools_response.tools.len(), "Retrieved tools");
        Ok(tools_response.tools)
    }

    /// Call a tool
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Value,
    ) -> McpClientResult<Vec<ToolResult>> {
        debug!(tool = name, "Calling tool");

        let request = json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "id": self.next_request_id(),
            "params": {
                "name": name,
                "arguments": arguments
            }
        });

        let response = self.send_request_internal(request).await?;
        let call_response: CallToolResult =
            serde_json::from_value(response.get("result").cloned().unwrap_or(Value::Null))?;

        debug!(
            tool = name,
            is_error = call_response.is_error,
            "Tool call completed"
        );
        Ok(call_response.content)
    }

    /// List available resources
    pub async fn list_resources(&self) -> McpClientResult<Vec<Resource>> {
        debug!("Listing resources");

        let request = json!({
            "jsonrpc": "2.0",
            "method": "resources/list",
            "id": self.next_request_id(),
            "params": {}
        });

        let response = self.send_request_internal(request).await?;
        let resources_response: ListResourcesResult =
            serde_json::from_value(response.get("result").cloned().unwrap_or(Value::Null))?;

        debug!(
            count = resources_response.resources.len(),
            "Retrieved resources"
        );
        Ok(resources_response.resources)
    }

    /// Read a resource
    pub async fn read_resource(&self, uri: &str) -> McpClientResult<Vec<turul_mcp_protocol_2025_06_18::ResourceContent>> {
        debug!(uri = uri, "Reading resource");

        let request = json!({
            "jsonrpc": "2.0",
            "method": "resources/read",
            "id": self.next_request_id(),
            "params": {
                "uri": uri
            }
        });

        let response = self.send_request_internal(request).await?;
        let read_response: ReadResourceResult =
            serde_json::from_value(response.get("result").cloned().unwrap_or(Value::Null))?;

        debug!(
            uri = uri,
            content_count = read_response.contents.len(),
            "Resource read completed"
        );
        Ok(read_response.contents)
    }

    /// List available prompts
    pub async fn list_prompts(&self) -> McpClientResult<Vec<Prompt>> {
        debug!("Listing prompts");

        let request = json!({
            "jsonrpc": "2.0",
            "method": "prompts/list",
            "id": self.next_request_id(),
            "params": {}
        });

        let response = self.send_request_internal(request).await?;
        let prompts_response: ListPromptsResult =
            serde_json::from_value(response.get("result").cloned().unwrap_or(Value::Null))?;

        debug!(count = prompts_response.prompts.len(), "Retrieved prompts");
        Ok(prompts_response.prompts)
    }

    /// Get a prompt
    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<Value>,
    ) -> McpClientResult<Vec<turul_mcp_protocol_2025_06_18::PromptMessage>> {
        debug!(prompt = name, "Getting prompt");

        let mut params = json!({
            "name": name
        });

        if let Some(args) = arguments {
            params["arguments"] = args;
        }

        let request = json!({
            "jsonrpc": "2.0",
            "method": "prompts/get",
            "id": self.next_request_id(),
            "params": params
        });

        let response = self.send_request_internal(request).await?;
        let prompt_response: GetPromptResult =
            serde_json::from_value(response.get("result").cloned().unwrap_or(Value::Null))?;

        debug!(
            prompt = name,
            message_count = prompt_response.messages.len(),
            "Prompt retrieved"
        );
        Ok(prompt_response.messages)
    }

    /// Send a ping to test connectivity
    pub async fn ping(&self) -> McpClientResult<()> {
        debug!("Sending ping");

        let request = json!({
            "jsonrpc": "2.0",
            "method": "ping",
            "id": self.next_request_id(),
            "params": {}
        });

        self.send_request_internal(request).await?;
        debug!("Ping successful");
        Ok(())
    }

    /// Get stream handler for event callbacks
    pub async fn stream_handler(&self) -> tokio::sync::MutexGuard<'_, StreamHandler> {
        self.stream_handler.lock().await
    }

    /// Get session information
    pub async fn session_info(&self) -> crate::session::SessionInfo {
        self.session.session_info().await
    }

    /// Get transport statistics
    pub async fn transport_stats(&self) -> crate::transport::TransportStatistics {
        let transport = self.transport.lock().await;
        transport.statistics()
    }
}

/// Connection status information
#[derive(Debug, Clone)]
pub struct ConnectionStatus {
    pub transport_connected: bool,
    pub session_state: SessionState,
    pub transport_type: crate::transport::TransportType,
    pub endpoint: String,
    pub session_id: String,
    pub protocol_version: Option<String>,
}

impl ConnectionStatus {
    /// Check if fully connected and ready
    pub fn is_ready(&self) -> bool {
        self.transport_connected && matches!(self.session_state, SessionState::Active)
    }

    /// Get status summary
    pub fn summary(&self) -> String {
        format!(
            "{} transport to {} - Session {} ({})",
            self.transport_type,
            self.endpoint,
            &self.session_id[..8],
            self.session_state
        )
    }
}

/// Builder for creating MCP clients
pub struct McpClientBuilder {
    transport: Option<BoxedTransport>,
    config: Option<ClientConfig>,
}

impl McpClientBuilder {
    /// Create a new client builder
    pub fn new() -> Self {
        Self {
            transport: None,
            config: None,
        }
    }

    /// Set transport
    pub fn with_transport(mut self, transport: BoxedTransport) -> Self {
        self.transport = Some(transport);
        self
    }

    /// Set transport from URL
    pub fn with_url(mut self, url: &str) -> McpClientResult<Self> {
        let transport = TransportFactory::from_url(url)?;
        self.transport = Some(transport);
        Ok(self)
    }

    /// Set configuration
    pub fn with_config(mut self, config: ClientConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Build the client
    pub fn build(self) -> McpClient {
        let transport = self
            .transport
            .expect("Transport must be set before building client");
        let config = self.config.unwrap_or_default();

        McpClient::new(transport, config)
    }
}

impl Default for McpClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::http::HttpTransport;

    #[tokio::test]
    async fn test_client_builder() {
        let transport = HttpTransport::new("http://localhost:8080/mcp").unwrap();
        let client = McpClientBuilder::new()
            .with_transport(Box::new(transport))
            .build();

        // Basic smoke test
        assert!(!client.is_ready().await);
    }

    #[test]
    fn test_connection_status() {
        let status = ConnectionStatus {
            transport_connected: true,
            session_state: SessionState::Active,
            transport_type: crate::transport::TransportType::Http,
            endpoint: "http://localhost:8080/mcp".to_string(),
            session_id: "session123".to_string(),
            protocol_version: Some("2025-06-18".to_string()),
        };

        assert!(status.is_ready());
        assert!(status.summary().contains("HTTP transport"));
    }
}
