//! Main MCP client implementation

use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::{debug, info, warn};

use crate::config::ClientConfig;
use crate::error::{McpClientError, McpClientResult, SessionError};
use crate::session::{SessionManager, SessionState};
use crate::streaming::StreamHandler;
use crate::transport::BoxedTransport;

// Re-export protocol types for convenience
use turul_mcp_protocol::meta::Cursor;
use turul_mcp_protocol::resources::{ListResourceTemplatesResult, ResourceTemplate};
use turul_mcp_protocol::tasks::{
    CancelTaskResult, CreateTaskResult, GetTaskResult, ListTasksResult, Task,
};
use turul_mcp_protocol::{
    CallToolResult, GetPromptResult, InitializeResult, ListPromptsResult, ListResourcesResult,
    ListToolsResult, Prompt, ReadResourceResult, Resource, Tool,
};

/// Callback type for receiving server notifications.
///
/// The callback receives the notification method (e.g., `"notifications/tools/list_changed"`)
/// and the optional params object.
pub type NotificationCallback = Arc<dyn Fn(&str, Option<&Value>) + Send + Sync>;

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
    /// Handle for the response consumer task (sends JSON-RPC responses back to server)
    response_consumer_handle: Arc<parking_lot::Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Cached tool list (invalidated by `notifications/tools/list_changed`)
    cached_tools: Arc<RwLock<Option<Vec<Tool>>>>,
    /// Cached resource list (invalidated by `notifications/resources/list_changed`)
    cached_resources: Arc<RwLock<Option<Vec<Resource>>>>,
    /// Cached prompt list (invalidated by `notifications/prompts/list_changed`)
    cached_prompts: Arc<RwLock<Option<Vec<Prompt>>>>,
    /// User-supplied notification callback
    notification_callback: Option<NotificationCallback>,
}

impl Drop for McpClient {
    /// Automatic cleanup when client is dropped
    ///
    /// This ensures that if the client is dropped without explicit disconnect,
    /// we still attempt to send a DELETE request to clean up the session on the server.
    fn drop(&mut self) {
        // Abort response consumer task
        if let Some(handle) = self.response_consumer_handle.lock().take() {
            handle.abort();
        }

        let session_id = self.session.clone();
        let transport = self.transport.clone();

        // Spawn a background task to handle cleanup
        // We can't await in Drop, so we spawn a task that will complete cleanup
        tokio::spawn(async move {
            // Only send DELETE if we have a session ID
            if let Some(session_id_str) = session_id.session_id_optional().await {
                let mut transport_guard = transport.lock().await;

                info!(
                    session_id = session_id_str,
                    "McpClient dropped - attempting session cleanup via DELETE request"
                );

                if let Err(e) = transport_guard.send_delete(&session_id_str).await {
                    warn!(
                        session_id = session_id_str,
                        error = %e,
                        "Failed to send DELETE request during Drop cleanup"
                    );
                } else {
                    info!(
                        session_id = session_id_str,
                        "Successfully sent DELETE request during Drop cleanup"
                    );
                }
            } else {
                debug!("No session ID available, skipping DELETE request during Drop");
            }

            // Also terminate the session locally
            session_id
                .terminate(Some("Client dropped".to_string()))
                .await;
        });
    }
}

impl McpClient {
    /// Create a new MCP client with the given transport
    pub fn new(transport: BoxedTransport, config: ClientConfig) -> Self {
        Self::new_with_callback(transport, config, None)
    }

    /// Create a new MCP client with an optional notification callback
    fn new_with_callback(
        transport: BoxedTransport,
        config: ClientConfig,
        notification_callback: Option<NotificationCallback>,
    ) -> Self {
        let session = Arc::new(SessionManager::new(config.clone()));

        Self {
            transport: Arc::new(tokio::sync::Mutex::new(transport)),
            session,
            config,
            stream_handler: Arc::new(tokio::sync::Mutex::new(StreamHandler::new())),
            request_counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            response_consumer_handle: Arc::new(parking_lot::Mutex::new(None)),
            cached_tools: Arc::new(RwLock::new(None)),
            cached_resources: Arc::new(RwLock::new(None)),
            cached_prompts: Arc::new(RwLock::new(None)),
            notification_callback,
        }
    }

    /// Connect to the MCP server
    pub async fn connect(&self) -> McpClientResult<()> {
        info!("Connecting to MCP server");

        // Abort any existing response consumer before reconnecting
        if let Some(handle) = self.response_consumer_handle.lock().take() {
            handle.abort();
        }

        // Connect transport
        {
            let mut transport = self.transport.lock().await;
            transport.connect().await?;

            // Start event listener if supported
            if transport.capabilities().server_events {
                let receiver = transport.start_event_listener().await?;

                // Create response channel for sending JSON-RPC responses back
                let (response_tx, mut response_rx) =
                    tokio::sync::mpsc::unbounded_channel::<serde_json::Value>();

                let mut stream_handler = self.stream_handler.lock().await;
                stream_handler.set_receiver(receiver);
                stream_handler.set_response_sender(response_tx);

                // Register internal notification handler for list_changed signals
                {
                    let cached_tools = Arc::clone(&self.cached_tools);
                    let cached_resources = Arc::clone(&self.cached_resources);
                    let cached_prompts = Arc::clone(&self.cached_prompts);
                    let user_callback = self.notification_callback.clone();

                    stream_handler.on_notification(move |notification| {
                        let method = notification
                            .get("method")
                            .and_then(|m| m.as_str())
                            .unwrap_or("");
                        let params = notification.get("params");

                        match method {
                            "notifications/tools/list_changed" => {
                                info!("Server sent notifications/tools/list_changed — invalidating tool cache");
                                // Invalidate synchronously using try_write to avoid async in sync callback
                                if let Ok(mut cache) = cached_tools.try_write() {
                                    *cache = None;
                                } else {
                                    warn!("Could not acquire tool cache write lock for invalidation");
                                }
                            }
                            "notifications/resources/list_changed" => {
                                info!("Server sent notifications/resources/list_changed — invalidating resource cache");
                                if let Ok(mut cache) = cached_resources.try_write() {
                                    *cache = None;
                                } else {
                                    warn!("Could not acquire resource cache write lock for invalidation");
                                }
                            }
                            "notifications/prompts/list_changed" => {
                                info!("Server sent notifications/prompts/list_changed — invalidating prompt cache");
                                if let Ok(mut cache) = cached_prompts.try_write() {
                                    *cache = None;
                                } else {
                                    warn!("Could not acquire prompt cache write lock for invalidation");
                                }
                            }
                            _ => {
                                debug!(method = method, "Received server notification");
                            }
                        }

                        // Forward to user callback if registered
                        if let Some(ref cb) = user_callback {
                            cb(method, params);
                        }
                    });
                }

                stream_handler.start().await?;

                // Spawn consumer task that drains the channel and sends responses
                let transport_clone = Arc::clone(&self.transport);
                let consumer_handle = tokio::spawn(async move {
                    while let Some(response) = response_rx.recv().await {
                        let mut transport = transport_clone.lock().await;
                        if let Err(e) = transport.send_notification(response).await {
                            warn!("Failed to send response back to server: {}", e);
                        }
                    }
                    debug!("Response consumer task stopped");
                });

                *self.response_consumer_handle.lock() = Some(consumer_handle);
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

        // Stop response consumer task
        if let Some(handle) = self.response_consumer_handle.lock().take() {
            handle.abort();
        }

        // Send DELETE request for session cleanup if we have a session ID
        if let Some(session_id) = self.session.session_id_optional().await {
            let mut transport = self.transport.lock().await;
            if let Err(e) = transport.send_delete(&session_id).await {
                warn!(
                    session_id = session_id,
                    error = %e,
                    "Failed to send DELETE request during disconnect - continuing with cleanup"
                );
            }
        } else {
            debug!("No session ID available, skipping DELETE request during disconnect");
        }

        // Terminate session locally
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

        // Send initialize request with timeout (need headers for session ID)
        let response = timeout(
            self.config.timeouts.initialization,
            self.send_request_with_headers_internal(json_rpc_request),
        )
        .await
        .map_err(|_| McpClientError::Timeout)?;

        let transport_response = response?;

        // Extract session ID from headers (MCP protocol) - case insensitive lookup
        let session_id = transport_response
            .headers
            .iter()
            .find(|(key, _)| key.to_lowercase() == "mcp-session-id")
            .map(|(_, value)| value.clone());

        if let Some(session_id) = session_id {
            info!("Server provided session ID: {}", session_id);

            // Store in session manager
            self.session.set_session_id(session_id.clone()).await?;

            // Tell transport to include session ID in all subsequent requests
            let mut transport = self.transport.lock().await;
            transport.set_session_id(session_id);
        } else {
            debug!("Server did not provide Mcp-Session-Id — stateless session (spec-valid)");
        }

        // Parse initialize response
        let init_response: InitializeResult = serde_json::from_value(
            transport_response
                .body
                .get("result")
                .cloned()
                .unwrap_or(Value::Null),
        )
        .map_err(|e| {
            McpClientError::generic(format!("Failed to parse initialize response: {}", e))
        })?;

        // Validate negotiated protocol version
        SessionManager::validate_protocol_version(&init_response.protocol_version)?;

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

        // Send initialized notification per MCP spec
        let initialized_notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
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

                    // MCP spec: 404 means session unknown — must re-initialize
                    if e.is_session_expired() {
                        warn!("Session expired (HTTP 404) — attempting re-initialization");
                        self.session.reset().await;
                        // Clear stale session ID from transport so initialize
                        // request is sent without Mcp-Session-Id header
                        {
                            let mut transport = self.transport.lock().await;
                            transport.clear_session_id();
                        }
                        if let Err(reinit_err) = self.initialize_session().await {
                            warn!(error = %reinit_err, "Re-initialization failed");
                            return Err(e);
                        }
                        // Retry the request with the new session
                        continue;
                    }

                    // JSON-RPC -32031: server rejected because notifications/initialized
                    // hasn't been processed yet. Disconnect, full reconnect, retry once.
                    if e.is_session_not_initialized() {
                        warn!(
                            "Session not initialized (code -32031) — \
                             disconnecting and reconnecting"
                        );
                        // Full reconnect: disconnect old session, connect fresh
                        if let Err(dc_err) = self.disconnect().await {
                            warn!(error = %dc_err, "Disconnect during session retry failed");
                        }
                        self.session.reset().await;
                        if let Err(reconnect_err) = self.connect().await {
                            warn!(error = %reconnect_err, "Reconnect after -32031 failed");
                            return Err(e);
                        }
                        // Retry the original request exactly once
                        return match self.send_request_raw(request).await {
                            Ok(response) => {
                                self.session.update_activity().await;
                                Ok(response)
                            }
                            Err(retry_err) => {
                                warn!(error = %retry_err, "Retry after reconnect also failed");
                                Err(retry_err)
                            }
                        };
                    }

                    if !e.is_retryable() || !self.config.retry.should_retry(attempt + 1) {
                        return Err(e);
                    }

                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| McpClientError::generic("All retry attempts failed")))
    }

    /// Send request with headers and handle retries (used for initialization)
    async fn send_request_with_headers_internal(
        &self,
        request: Value,
    ) -> McpClientResult<crate::transport::TransportResponse> {
        let mut last_error = None;

        for attempt in 0..self.config.retry.max_attempts {
            if attempt > 0 {
                let delay = self.config.retry.delay_for_attempt(attempt);
                debug!(
                    attempt = attempt,
                    delay_ms = delay.as_millis(),
                    "Retrying request with headers"
                );
                tokio::time::sleep(delay).await;
            }

            match self.send_request_with_headers_raw(request.clone()).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    warn!(
                        attempt = attempt,
                        max_attempts = self.config.retry.max_attempts,
                        error = %e,
                        "Request with headers failed"
                    );

                    if !e.is_retryable() {
                        return Err(e);
                    }

                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| McpClientError::generic("All retry attempts failed")))
    }

    /// Send raw request with headers without retries
    async fn send_request_with_headers_raw(
        &self,
        request: Value,
    ) -> McpClientResult<crate::transport::TransportResponse> {
        let mut transport = self.transport.lock().await;

        timeout(
            self.config.timeouts.request,
            transport.send_request_with_headers(request),
        )
        .await
        .map_err(|_| McpClientError::Timeout)?
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

    /// List available tools (returns cached result if available)
    ///
    /// The cache is automatically invalidated when the server sends a
    /// `notifications/tools/list_changed` notification. Use [`refresh_tools`](Self::refresh_tools)
    /// to force a fresh fetch.
    pub async fn list_tools(&self) -> McpClientResult<Vec<Tool>> {
        // Return cached tools if available
        {
            let cache = self.cached_tools.read().await;
            if let Some(ref tools) = *cache {
                debug!(count = tools.len(), "Returning cached tools");
                return Ok(tools.clone());
            }
        }

        let tools = self.fetch_tools().await?;

        // Update cache
        {
            let mut cache = self.cached_tools.write().await;
            *cache = Some(tools.clone());
        }

        Ok(tools)
    }

    /// Force a fresh fetch of tools from the server, bypassing and updating the cache.
    pub async fn refresh_tools(&self) -> McpClientResult<Vec<Tool>> {
        let tools = self.fetch_tools().await?;

        // Update cache
        {
            let mut cache = self.cached_tools.write().await;
            *cache = Some(tools.clone());
        }

        Ok(tools)
    }

    /// Fetch tools from the server (no cache interaction).
    async fn fetch_tools(&self) -> McpClientResult<Vec<Tool>> {
        debug!("Fetching tools from server");

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

    /// List available tools with pagination support
    pub async fn list_tools_paginated(
        &self,
        cursor: Option<Cursor>,
    ) -> McpClientResult<ListToolsResult> {
        debug!("Listing tools with pagination");

        let request_params = if let Some(cursor) = cursor {
            json!({ "cursor": cursor.as_str() })
        } else {
            json!({})
        };

        let request = json!({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": self.next_request_id(),
            "params": request_params
        });

        let response = self.send_request_internal(request).await?;
        let tools_response: ListToolsResult =
            serde_json::from_value(response.get("result").cloned().unwrap_or(Value::Null))?;

        debug!(
            count = tools_response.tools.len(),
            has_cursor = tools_response.next_cursor.is_some(),
            "Retrieved tools with pagination"
        );
        Ok(tools_response)
    }

    /// Call a tool
    pub async fn call_tool(&self, name: &str, arguments: Value) -> McpClientResult<CallToolResult> {
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
        Ok(call_response)
    }

    /// List available resources (returns cached result if available)
    ///
    /// The cache is automatically invalidated when the server sends a
    /// `notifications/resources/list_changed` notification. Use
    /// [`refresh_resources`](Self::refresh_resources) to force a fresh fetch.
    pub async fn list_resources(&self) -> McpClientResult<Vec<Resource>> {
        {
            let cache = self.cached_resources.read().await;
            if let Some(ref resources) = *cache {
                debug!(count = resources.len(), "Returning cached resources");
                return Ok(resources.clone());
            }
        }

        let resources = self.fetch_resources().await?;
        {
            let mut cache = self.cached_resources.write().await;
            *cache = Some(resources.clone());
        }
        Ok(resources)
    }

    /// Force a fresh fetch of resources from the server, bypassing and updating the cache.
    pub async fn refresh_resources(&self) -> McpClientResult<Vec<Resource>> {
        let resources = self.fetch_resources().await?;
        {
            let mut cache = self.cached_resources.write().await;
            *cache = Some(resources.clone());
        }
        Ok(resources)
    }

    /// Fetch resources from the server (no cache interaction).
    async fn fetch_resources(&self) -> McpClientResult<Vec<Resource>> {
        debug!("Fetching resources from server");

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

    /// List available resources with pagination support
    pub async fn list_resources_paginated(
        &self,
        cursor: Option<Cursor>,
    ) -> McpClientResult<ListResourcesResult> {
        debug!("Listing resources with pagination");

        let request_params = if let Some(cursor) = cursor {
            json!({ "cursor": cursor.as_str() })
        } else {
            json!({})
        };

        let request = json!({
            "jsonrpc": "2.0",
            "method": "resources/list",
            "id": self.next_request_id(),
            "params": request_params
        });

        let response = self.send_request_internal(request).await?;
        let resources_response: ListResourcesResult =
            serde_json::from_value(response.get("result").cloned().unwrap_or(Value::Null))?;

        debug!(
            count = resources_response.resources.len(),
            has_cursor = resources_response.next_cursor.is_some(),
            "Retrieved resources with pagination"
        );
        Ok(resources_response)
    }

    /// Read a resource
    pub async fn read_resource(
        &self,
        uri: &str,
    ) -> McpClientResult<Vec<turul_mcp_protocol::ResourceContent>> {
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

    /// List available resource templates
    pub async fn list_resource_templates(&self) -> McpClientResult<Vec<ResourceTemplate>> {
        debug!("Listing resource templates");

        let request = json!({
            "jsonrpc": "2.0",
            "method": "resources/templates/list",
            "id": self.next_request_id(),
            "params": {}
        });

        let response = self.send_request_internal(request).await?;
        let templates_response: ListResourceTemplatesResult =
            serde_json::from_value(response.get("result").cloned().unwrap_or(Value::Null))?;

        debug!(
            count = templates_response.resource_templates.len(),
            "Retrieved resource templates"
        );
        Ok(templates_response.resource_templates)
    }

    /// List available resource templates with pagination support
    pub async fn list_resource_templates_paginated(
        &self,
        cursor: Option<Cursor>,
    ) -> McpClientResult<ListResourceTemplatesResult> {
        debug!("Listing resource templates with pagination");

        let request_params = if let Some(cursor) = cursor {
            json!({ "cursor": cursor.as_str() })
        } else {
            json!({})
        };

        let request = json!({
            "jsonrpc": "2.0",
            "method": "resources/templates/list",
            "id": self.next_request_id(),
            "params": request_params
        });

        let response = self.send_request_internal(request).await?;
        let templates_response: ListResourceTemplatesResult =
            serde_json::from_value(response.get("result").cloned().unwrap_or(Value::Null))?;

        debug!(
            count = templates_response.resource_templates.len(),
            has_cursor = templates_response.next_cursor.is_some(),
            "Retrieved resource templates with pagination"
        );
        Ok(templates_response)
    }

    /// List available prompts (returns cached result if available)
    ///
    /// The cache is automatically invalidated when the server sends a
    /// `notifications/prompts/list_changed` notification. Use
    /// [`refresh_prompts`](Self::refresh_prompts) to force a fresh fetch.
    pub async fn list_prompts(&self) -> McpClientResult<Vec<Prompt>> {
        {
            let cache = self.cached_prompts.read().await;
            if let Some(ref prompts) = *cache {
                debug!(count = prompts.len(), "Returning cached prompts");
                return Ok(prompts.clone());
            }
        }

        let prompts = self.fetch_prompts().await?;
        {
            let mut cache = self.cached_prompts.write().await;
            *cache = Some(prompts.clone());
        }
        Ok(prompts)
    }

    /// Force a fresh fetch of prompts from the server, bypassing and updating the cache.
    pub async fn refresh_prompts(&self) -> McpClientResult<Vec<Prompt>> {
        let prompts = self.fetch_prompts().await?;
        {
            let mut cache = self.cached_prompts.write().await;
            *cache = Some(prompts.clone());
        }
        Ok(prompts)
    }

    /// Fetch prompts from the server (no cache interaction).
    async fn fetch_prompts(&self) -> McpClientResult<Vec<Prompt>> {
        debug!("Fetching prompts from server");

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

    /// List available prompts with pagination support
    pub async fn list_prompts_paginated(
        &self,
        cursor: Option<Cursor>,
    ) -> McpClientResult<ListPromptsResult> {
        debug!("Listing prompts with pagination");

        let request_params = if let Some(cursor) = cursor {
            json!({ "cursor": cursor.as_str() })
        } else {
            json!({})
        };

        let request = json!({
            "jsonrpc": "2.0",
            "method": "prompts/list",
            "id": self.next_request_id(),
            "params": request_params
        });

        let response = self.send_request_internal(request).await?;
        let prompts_response: ListPromptsResult =
            serde_json::from_value(response.get("result").cloned().unwrap_or(Value::Null))?;

        debug!(
            count = prompts_response.prompts.len(),
            has_cursor = prompts_response.next_cursor.is_some(),
            "Retrieved prompts with pagination"
        );
        Ok(prompts_response)
    }

    /// Get a prompt
    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<Value>,
    ) -> McpClientResult<GetPromptResult> {
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
        Ok(prompt_response)
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

    // === Task Operations ===

    /// Get a task by ID
    pub async fn get_task(&self, task_id: &str) -> McpClientResult<Task> {
        debug!(task_id = task_id, "Getting task");

        let request = json!({
            "jsonrpc": "2.0",
            "method": "tasks/get",
            "id": self.next_request_id(),
            "params": {
                "taskId": task_id
            }
        });

        let response = self.send_request_internal(request).await?;
        let task_response: GetTaskResult =
            serde_json::from_value(response.get("result").cloned().unwrap_or(Value::Null))?;

        debug!(task_id = task_id, status = ?task_response.task.status, "Task retrieved");
        Ok(task_response.task)
    }

    /// List tasks
    pub async fn list_tasks(&self) -> McpClientResult<Vec<Task>> {
        debug!("Listing tasks");

        let request = json!({
            "jsonrpc": "2.0",
            "method": "tasks/list",
            "id": self.next_request_id(),
            "params": {}
        });

        let response = self.send_request_internal(request).await?;
        let tasks_response: ListTasksResult =
            serde_json::from_value(response.get("result").cloned().unwrap_or(Value::Null))?;

        debug!(count = tasks_response.tasks.len(), "Retrieved tasks");
        Ok(tasks_response.tasks)
    }

    /// List tasks with pagination support
    pub async fn list_tasks_paginated(
        &self,
        cursor: Option<Cursor>,
    ) -> McpClientResult<ListTasksResult> {
        debug!("Listing tasks with pagination");

        let request_params = if let Some(cursor) = cursor {
            json!({ "cursor": cursor.as_str() })
        } else {
            json!({})
        };

        let request = json!({
            "jsonrpc": "2.0",
            "method": "tasks/list",
            "id": self.next_request_id(),
            "params": request_params
        });

        let response = self.send_request_internal(request).await?;
        let tasks_response: ListTasksResult =
            serde_json::from_value(response.get("result").cloned().unwrap_or(Value::Null))?;

        debug!(
            count = tasks_response.tasks.len(),
            has_cursor = tasks_response.next_cursor.is_some(),
            "Retrieved tasks with pagination"
        );
        Ok(tasks_response)
    }

    /// Cancel a task
    pub async fn cancel_task(&self, task_id: &str) -> McpClientResult<Task> {
        debug!(task_id = task_id, "Cancelling task");

        let request = json!({
            "jsonrpc": "2.0",
            "method": "tasks/cancel",
            "id": self.next_request_id(),
            "params": {
                "taskId": task_id
            }
        });

        let response = self.send_request_internal(request).await?;
        let cancel_response: CancelTaskResult =
            serde_json::from_value(response.get("result").cloned().unwrap_or(Value::Null))?;

        debug!(task_id = task_id, status = ?cancel_response.task.status, "Task cancelled");
        Ok(cancel_response.task)
    }

    /// Get a task's result (blocks until the task reaches terminal status)
    ///
    /// Per MCP spec, if the task is still in progress the server will hold the
    /// response until it completes. Use a longer timeout for this operation.
    pub async fn get_task_result(&self, task_id: &str) -> McpClientResult<Value> {
        debug!(task_id = task_id, "Getting task result");

        let request = json!({
            "jsonrpc": "2.0",
            "method": "tasks/result",
            "id": self.next_request_id(),
            "params": {
                "taskId": task_id
            }
        });

        // Use long_operation timeout since tasks/result blocks until terminal
        let response = timeout(
            self.config.timeouts.long_operation,
            self.send_request_internal(request),
        )
        .await
        .map_err(|_| McpClientError::Timeout)??;

        let result = response.get("result").cloned().unwrap_or(Value::Null);
        debug!(task_id = task_id, "Task result retrieved");
        Ok(result)
    }

    /// Call a tool with task augmentation
    ///
    /// If the server supports tasks for this tool, it returns a `Task` (the tool
    /// executes asynchronously). Otherwise, it returns the normal `CallToolResult`.
    pub async fn call_tool_with_task(
        &self,
        name: &str,
        arguments: Value,
        ttl_ms: Option<i64>,
    ) -> McpClientResult<ToolCallResponse> {
        debug!(tool = name, "Calling tool with task augmentation");

        let mut params = json!({
            "name": name,
            "arguments": arguments,
            "task": {}
        });

        if let Some(ttl) = ttl_ms {
            params["task"]["ttl"] = json!(ttl);
        }

        let request = json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "id": self.next_request_id(),
            "params": params
        });

        let response = self.send_request_internal(request).await?;
        let result = response.get("result").cloned().unwrap_or(Value::Null);

        // Distinguish response type: CreateTaskResult has a "task" field,
        // CallToolResult has a "content" field
        if result.get("task").is_some() {
            let task_result: CreateTaskResult = serde_json::from_value(result)?;
            debug!(
                tool = name,
                task_id = task_result.task.task_id,
                "Tool call created task"
            );
            Ok(ToolCallResponse::TaskCreated(task_result.task))
        } else {
            let call_result: CallToolResult = serde_json::from_value(result)?;
            debug!(
                tool = name,
                is_error = call_result.is_error,
                "Tool call completed synchronously"
            );
            Ok(ToolCallResponse::Immediate(call_result))
        }
    }

    /// Get stream handler for event callbacks
    pub async fn stream_handler(&self) -> tokio::sync::MutexGuard<'_, StreamHandler> {
        self.stream_handler.lock().await
    }

    /// Invalidate all cached lists (tools, resources, prompts).
    ///
    /// The next call to `list_tools()`, `list_resources()`, or `list_prompts()`
    /// will fetch fresh data from the server.
    pub async fn invalidate_caches(&self) {
        *self.cached_tools.write().await = None;
        *self.cached_resources.write().await = None;
        *self.cached_prompts.write().await = None;
        debug!("All list caches invalidated");
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
    pub session_id: Option<String>,
    pub protocol_version: Option<String>,
}

impl ConnectionStatus {
    /// Check if fully connected and ready
    pub fn is_ready(&self) -> bool {
        self.transport_connected && matches!(self.session_state, SessionState::Active)
    }

    /// Get status summary
    pub fn summary(&self) -> String {
        let session_display = match &self.session_id {
            Some(id) => &id[..id.len().min(8)],
            None => "None",
        };
        format!(
            "{} transport to {} - Session {} ({})",
            self.transport_type, self.endpoint, session_display, self.session_state
        )
    }
}

/// Response from a task-augmented tool call
///
/// When calling a tool with task augmentation, the server may either:
/// - Create a task and return it immediately (tool runs async)
/// - Execute the tool synchronously (if tasks not supported for this tool)
#[derive(Debug)]
pub enum ToolCallResponse {
    /// The tool executed synchronously and returned its result directly
    Immediate(CallToolResult),
    /// The server created a task — poll with `get_task()` or await with `get_task_result()`
    TaskCreated(Task),
}

impl ToolCallResponse {
    /// Returns `true` if the server created a task for this call
    pub fn is_task(&self) -> bool {
        matches!(self, ToolCallResponse::TaskCreated(_))
    }

    /// Returns the task if one was created
    pub fn task(&self) -> Option<&Task> {
        match self {
            ToolCallResponse::TaskCreated(task) => Some(task),
            _ => None,
        }
    }

    /// Returns the immediate result if the tool executed synchronously
    pub fn immediate_result(&self) -> Option<&CallToolResult> {
        match self {
            ToolCallResponse::Immediate(result) => Some(result),
            _ => None,
        }
    }
}

/// Builder for creating MCP clients
pub struct McpClientBuilder {
    transport: Option<BoxedTransport>,
    url: Option<String>,
    config: Option<ClientConfig>,
    notification_callback: Option<NotificationCallback>,
}

impl McpClientBuilder {
    /// Create a new client builder
    pub fn new() -> Self {
        Self {
            transport: None,
            url: None,
            config: None,
            notification_callback: None,
        }
    }

    /// Set transport directly (ConnectionConfig is NOT applied — caller owns the transport)
    pub fn with_transport(mut self, transport: BoxedTransport) -> Self {
        self.transport = Some(transport);
        self.url = None; // explicit transport overrides URL
        self
    }

    /// Set transport endpoint URL (transport constructed lazily in `build()` with config applied)
    ///
    /// Can be called before or after `with_config()` — config is applied at build time.
    pub fn with_url(mut self, url: &str) -> McpClientResult<Self> {
        // Validate URL eagerly so errors surface at call site
        let parsed = url::Url::parse(url).map_err(|e| {
            crate::error::TransportError::ConnectionFailed(format!("Invalid URL: {}", e))
        })?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(crate::error::TransportError::ConnectionFailed(format!(
                "Invalid scheme: {}",
                parsed.scheme()
            ))
            .into());
        }
        self.url = Some(url.to_string());
        self.transport = None; // URL overrides explicit transport
        Ok(self)
    }

    /// Set configuration (applied to transport at build time if using `with_url`)
    pub fn with_config(mut self, config: ClientConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Register a callback for server notifications.
    ///
    /// The callback fires for every server notification, including
    /// `notifications/tools/list_changed`, `notifications/resources/list_changed`,
    /// and `notifications/prompts/list_changed`.
    ///
    /// Note: The built-in cache invalidation for `list_changed` notifications
    /// always runs regardless of whether a user callback is registered.
    pub fn on_notification<F>(mut self, callback: F) -> Self
    where
        F: Fn(&str, Option<&Value>) + Send + Sync + 'static,
    {
        self.notification_callback = Some(Arc::new(callback));
        self
    }

    /// Build the client
    ///
    /// If `with_url()` was used, the transport is constructed here with `ConnectionConfig` applied.
    /// If `with_transport()` was used, the provided transport is used as-is.
    pub fn build(self) -> McpClient {
        let config = self.config.unwrap_or_default();
        let transport = if let Some(transport) = self.transport {
            transport
        } else if let Some(ref url) = self.url {
            // Detect transport type from URL, then construct with config applied
            let transport_type = crate::transport::detect_transport_type(url)
                .expect("URL was validated in with_url() but detection failed");
            match transport_type {
                crate::transport::TransportType::Http => Box::new(
                    crate::transport::http::HttpTransport::with_config(url, &config.connection)
                        .expect(
                            "URL was validated in with_url() but transport construction failed",
                        ),
                )
                    as crate::transport::BoxedTransport,
                crate::transport::TransportType::Sse => {
                    // SSE is a legacy transport — ConnectionConfig not wired (no with_config)
                    Box::new(
                        crate::transport::sse::SseTransport::new(url)
                            .expect("URL was validated in with_url() but SSE construction failed"),
                    )
                }
            }
        } else {
            panic!("Transport must be set via with_transport() or with_url() before building");
        };

        McpClient::new_with_callback(transport, config, self.notification_callback)
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
    use crate::transport::{
        ConnectionInfo, EventReceiver, ServerEvent, TransportCapabilities, TransportResponse,
        TransportStatistics, TransportType,
    };
    use async_trait::async_trait;
    use std::collections::HashMap;
    use tokio::sync::mpsc;

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
            session_id: Some("session123".to_string()),
            protocol_version: Some("2025-11-25".to_string()),
        };

        assert!(status.is_ready());
        assert!(status.summary().contains("HTTP transport"));
    }

    /// Mock transport that records send_notification() calls and provides
    /// a controllable event channel for injecting ServerEvents.
    struct MockTransport {
        event_tx: mpsc::UnboundedSender<ServerEvent>,
        event_rx: Option<mpsc::UnboundedReceiver<ServerEvent>>,
        notifications: Arc<tokio::sync::Mutex<Vec<Value>>>,
        connected: bool,
    }

    impl MockTransport {
        fn new() -> (Self, Arc<tokio::sync::Mutex<Vec<Value>>>) {
            let (event_tx, event_rx) = mpsc::unbounded_channel();
            let notifications = Arc::new(tokio::sync::Mutex::new(Vec::new()));
            let mock = Self {
                event_tx,
                event_rx: Some(event_rx),
                notifications: Arc::clone(&notifications),
                connected: false,
            };
            (mock, notifications)
        }

        /// Get the event sender for injecting server events from the test
        fn event_sender(&self) -> mpsc::UnboundedSender<ServerEvent> {
            self.event_tx.clone()
        }
    }

    #[async_trait]
    impl crate::transport::Transport for MockTransport {
        fn transport_type(&self) -> TransportType {
            TransportType::Http
        }

        fn capabilities(&self) -> TransportCapabilities {
            TransportCapabilities {
                streaming: true,
                bidirectional: true,
                server_events: true,
                max_message_size: None,
                persistent: true,
            }
        }

        async fn connect(&mut self) -> McpClientResult<()> {
            self.connected = true;
            Ok(())
        }

        async fn disconnect(&mut self) -> McpClientResult<()> {
            self.connected = false;
            Ok(())
        }

        fn is_connected(&self) -> bool {
            self.connected
        }

        async fn send_request(&mut self, _request: Value) -> McpClientResult<Value> {
            // Not used in this test path
            Ok(json!({"jsonrpc": "2.0", "result": {}}))
        }

        async fn send_request_with_headers(
            &mut self,
            _request: Value,
        ) -> McpClientResult<TransportResponse> {
            // Return a valid initialize response with session ID
            let mut headers = HashMap::new();
            headers.insert("mcp-session-id".to_string(), "mock-session-123".to_string());

            Ok(TransportResponse::new(
                json!({
                    "jsonrpc": "2.0",
                    "id": "req_0",
                    "result": {
                        "protocolVersion": "2025-11-25",
                        "capabilities": {
                            "tools": { "listChanged": true }
                        },
                        "serverInfo": {
                            "name": "mock-server",
                            "version": "1.0.0"
                        }
                    }
                }),
                headers,
            ))
        }

        async fn send_notification(&mut self, notification: Value) -> McpClientResult<()> {
            self.notifications.lock().await.push(notification);
            Ok(())
        }

        async fn send_delete(&mut self, _session_id: &str) -> McpClientResult<()> {
            Ok(())
        }

        fn set_session_id(&mut self, _session_id: String) {}

        fn clear_session_id(&mut self) {}

        async fn start_event_listener(&mut self) -> McpClientResult<EventReceiver> {
            self.event_rx
                .take()
                .ok_or_else(|| McpClientError::generic("Event listener already started"))
        }

        fn connection_info(&self) -> ConnectionInfo {
            ConnectionInfo {
                transport_type: TransportType::Http,
                endpoint: "mock://test".to_string(),
                connected: self.connected,
                capabilities: self.capabilities(),
                metadata: Value::Null,
            }
        }

        fn statistics(&self) -> TransportStatistics {
            TransportStatistics::default()
        }
    }

    /// Verifies the full McpClient pipeline: server request → StreamHandler callback
    /// → response channel → consumer task → transport.send_notification().
    #[tokio::test]
    async fn test_client_response_consumer_pipeline() {
        let (mock, notifications) = MockTransport::new();
        let event_sender = mock.event_sender();

        let client = McpClientBuilder::new()
            .with_transport(Box::new(mock))
            .build();

        // connect() wires up StreamHandler + consumer task + runs initialization
        client.connect().await.unwrap();

        // Register a request callback via the public API
        {
            let handler = client.stream_handler().await;
            handler.on_request(|request| {
                let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
                match method {
                    "sampling/createMessage" => Ok(json!({
                        "role": "assistant",
                        "content": { "type": "text", "text": "mock response" },
                        "model": "test-model"
                    })),
                    _ => Err(format!("Unsupported: {}", method)),
                }
            });
        }

        // Inject a server-initiated request through the event channel
        event_sender
            .send(ServerEvent::Request(json!({
                "jsonrpc": "2.0",
                "id": "srv-req-42",
                "method": "sampling/createMessage",
                "params": {
                    "messages": [{"role": "user", "content": {"type": "text", "text": "Hi"}}],
                    "maxTokens": 100
                }
            })))
            .unwrap();

        // Wait for the consumer task to forward the response to transport
        let response = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                let notifs = notifications.lock().await;
                // Skip the first notification (notifications/initialized from connect())
                let responses: Vec<&Value> =
                    notifs.iter().filter(|n| n.get("id").is_some()).collect();
                if !responses.is_empty() {
                    return responses[0].clone();
                }
                drop(notifs);
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("timed out waiting for response to reach transport");

        // Verify JSON-RPC 2.0 response structure
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], "srv-req-42");
        assert!(
            response.get("error").is_none(),
            "should not have error field"
        );
        assert_eq!(response["result"]["role"], "assistant");
        assert_eq!(response["result"]["model"], "test-model");

        client.disconnect().await.unwrap();
    }

    /// Same pipeline but with an error callback — verifies error responses reach transport.
    #[tokio::test]
    async fn test_client_response_consumer_pipeline_error() {
        let (mock, notifications) = MockTransport::new();
        let event_sender = mock.event_sender();

        let client = McpClientBuilder::new()
            .with_transport(Box::new(mock))
            .build();

        client.connect().await.unwrap();

        {
            let handler = client.stream_handler().await;
            handler.on_request(|_req| Err("not supported".to_string()));
        }

        event_sender
            .send(ServerEvent::Request(json!({
                "jsonrpc": "2.0",
                "id": 99,
                "method": "elicitation/create",
                "params": {}
            })))
            .unwrap();

        let response = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                let notifs = notifications.lock().await;
                let responses: Vec<&Value> =
                    notifs.iter().filter(|n| n.get("id").is_some()).collect();
                if !responses.is_empty() {
                    return responses[0].clone();
                }
                drop(notifs);
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("timed out waiting for error response");

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 99);
        assert_eq!(response["error"]["code"], -32603);
        assert!(
            response["error"]["message"]
                .as_str()
                .unwrap()
                .contains("not supported")
        );

        client.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_builder_with_sse_url_yields_sse_transport() {
        let client = McpClientBuilder::new()
            .with_url("http://localhost:9999/sse")
            .unwrap()
            .build();
        let status = client.connection_status().await;
        assert_eq!(
            status.transport_type,
            crate::transport::TransportType::Sse,
            "URL with /sse path must yield SSE transport"
        );
    }

    #[tokio::test]
    async fn test_builder_with_mcp_url_yields_http_transport() {
        let client = McpClientBuilder::new()
            .with_url("http://localhost:9999/mcp")
            .unwrap()
            .build();
        let status = client.connection_status().await;
        assert_eq!(
            status.transport_type,
            crate::transport::TransportType::Http,
            "Non-SSE URL must yield HTTP transport"
        );
    }

    // ── StatefulMockTransport ──────────────────────────────────────────

    use std::collections::VecDeque;
    use std::sync::atomic::AtomicU32;

    /// A mock transport that supports multi-step response sequences and call tracking.
    /// Unlike `MockTransport` (which returns fixed responses), this transport pops
    /// responses from queues, enabling tests that exercise full session lifecycles
    /// (initialize → set session → requests → re-initialize on 404 → …).
    #[allow(dead_code)]
    struct StatefulMockTransport {
        /// Sequence of responses for send_request_with_headers (initialize)
        init_responses: Arc<std::sync::Mutex<VecDeque<McpClientResult<TransportResponse>>>>,
        /// Sequence of responses for send_request (normal requests)
        request_responses: Arc<std::sync::Mutex<VecDeque<McpClientResult<Value>>>>,
        /// Tracks set_session_id calls in order
        set_session_ids: Arc<std::sync::Mutex<Vec<String>>>,
        /// Tracks clear_session_id call count
        clear_count: Arc<AtomicU32>,
        /// Event channel sender
        event_tx: Option<mpsc::UnboundedSender<ServerEvent>>,
        /// Event channel receiver (taken once by start_event_listener)
        event_rx: Option<mpsc::UnboundedReceiver<ServerEvent>>,
        /// Capabilities
        caps: TransportCapabilities,
        /// Connected flag
        connected: bool,
    }

    impl StatefulMockTransport {
        fn new() -> Self {
            let (event_tx, event_rx) = mpsc::unbounded_channel();
            Self {
                init_responses: Arc::new(std::sync::Mutex::new(VecDeque::new())),
                request_responses: Arc::new(std::sync::Mutex::new(VecDeque::new())),
                set_session_ids: Arc::new(std::sync::Mutex::new(Vec::new())),
                clear_count: Arc::new(AtomicU32::new(0)),
                event_tx: Some(event_tx),
                event_rx: Some(event_rx),
                caps: TransportCapabilities {
                    streaming: true,
                    bidirectional: true,
                    server_events: true,
                    max_message_size: None,
                    persistent: true,
                },
                connected: false,
            }
        }

        #[allow(dead_code)]
        fn push_init_response(&mut self, resp: McpClientResult<TransportResponse>) {
            self.init_responses.lock().unwrap().push_back(resp);
        }

        fn push_request_response(&mut self, resp: McpClientResult<Value>) {
            self.request_responses.lock().unwrap().push_back(resp);
        }

        /// Create a valid initialize TransportResponse with optional session ID header.
        #[allow(dead_code)]
        fn make_init_response(
            session_id: Option<&str>,
            protocol_version: &str,
        ) -> TransportResponse {
            let mut headers = HashMap::new();
            if let Some(sid) = session_id {
                headers.insert("mcp-session-id".to_string(), sid.to_string());
            }
            TransportResponse::new(
                json!({
                    "jsonrpc": "2.0",
                    "id": "req_0",
                    "result": {
                        "protocolVersion": protocol_version,
                        "capabilities": {
                            "tools": { "listChanged": true }
                        },
                        "serverInfo": {
                            "name": "stateful-mock",
                            "version": "1.0.0"
                        }
                    }
                }),
                headers,
            )
        }
    }

    #[async_trait]
    impl crate::transport::Transport for StatefulMockTransport {
        fn transport_type(&self) -> TransportType {
            TransportType::Http
        }

        fn capabilities(&self) -> TransportCapabilities {
            self.caps.clone()
        }

        async fn connect(&mut self) -> McpClientResult<()> {
            self.connected = true;
            Ok(())
        }

        async fn disconnect(&mut self) -> McpClientResult<()> {
            self.connected = false;
            Ok(())
        }

        fn is_connected(&self) -> bool {
            self.connected
        }

        async fn send_request(&mut self, _request: Value) -> McpClientResult<Value> {
            self.request_responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| {
                    Err(McpClientError::generic(
                        "StatefulMockTransport: no more request responses queued",
                    ))
                })
        }

        async fn send_request_with_headers(
            &mut self,
            _request: Value,
        ) -> McpClientResult<TransportResponse> {
            self.init_responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| {
                    Err(McpClientError::generic(
                        "StatefulMockTransport: no more init responses queued",
                    ))
                })
        }

        async fn send_notification(&mut self, _notification: Value) -> McpClientResult<()> {
            Ok(())
        }

        async fn send_delete(&mut self, _session_id: &str) -> McpClientResult<()> {
            Ok(())
        }

        fn set_session_id(&mut self, session_id: String) {
            self.set_session_ids.lock().unwrap().push(session_id);
        }

        fn clear_session_id(&mut self) {
            self.clear_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }

        async fn start_event_listener(&mut self) -> McpClientResult<EventReceiver> {
            self.event_rx
                .take()
                .ok_or_else(|| McpClientError::generic("Event listener already started"))
        }

        fn connection_info(&self) -> ConnectionInfo {
            ConnectionInfo {
                transport_type: TransportType::Http,
                endpoint: "stateful-mock://test".to_string(),
                connected: self.connected,
                capabilities: self.caps.clone(),
                metadata: Value::Null,
            }
        }

        fn statistics(&self) -> TransportStatistics {
            TransportStatistics::default()
        }
    }

    #[tokio::test]
    async fn test_stateful_mock_transport_sequences() {
        use crate::transport::Transport;

        let mut transport = StatefulMockTransport::new();
        transport.push_request_response(Ok(json!({"result": "first"})));
        transport.push_request_response(Ok(json!({"result": "second"})));

        let r1 = transport.send_request(json!({})).await.unwrap();
        assert_eq!(r1["result"], "first");
        let r2 = transport.send_request(json!({})).await.unwrap();
        assert_eq!(r2["result"], "second");

        // Queue exhausted → error
        assert!(transport.send_request(json!({})).await.is_err());
    }

    // ── Session lifecycle tests ─────────────────────────────────────────

    use crate::config::RetryConfig;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    /// Helper: build a fast-retry config (1 ms delays) with the given max_attempts.
    fn fast_retry_config(max_attempts: u32) -> ClientConfig {
        ClientConfig {
            retry: RetryConfig {
                max_attempts,
                initial_delay: Duration::from_millis(1),
                max_delay: Duration::from_millis(1),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Test 2.1 — 404 recovery path resets session, clears stale transport session ID,
    /// re-initializes with a fresh session, and retries the original request.
    #[tokio::test]
    async fn test_404_reinitialize_clears_stale_session_id() {
        let mut transport = StatefulMockTransport::new();

        // 1. Initial initialize → session "AAA"
        transport.push_init_response(Ok(StatefulMockTransport::make_init_response(
            Some("session-AAA"),
            "2025-11-25",
        )));

        // 2. First real request → 404 (session expired on server)
        transport.push_request_response(Err(McpClientError::Transport(
            crate::error::TransportError::HttpStatus {
                status: 404,
                message: "Not Found".to_string(),
            },
        )));

        // 3. Re-initialize → session "BBB"
        transport.push_init_response(Ok(StatefulMockTransport::make_init_response(
            Some("session-BBB"),
            "2025-11-25",
        )));

        // 4. Retry request → success
        transport.push_request_response(Ok(json!({
            "jsonrpc": "2.0",
            "id": "req_1",
            "result": { "tools": [] }
        })));

        let clear_count = transport.clear_count.clone();
        let set_ids = transport.set_session_ids.clone();

        let client = McpClient::new(Box::new(transport), fast_retry_config(3));
        client.connect().await.unwrap();

        // This should: fail with 404 → clear session → re-init → retry → succeed
        let result = client.list_tools().await;
        assert!(
            result.is_ok(),
            "Request should succeed after 404 re-initialization: {:?}",
            result.err()
        );

        // Verify clear_session_id was called exactly once during 404 recovery
        assert_eq!(
            clear_count.load(Ordering::SeqCst),
            1,
            "clear_session_id must be called exactly once during 404 recovery"
        );

        // Verify new session ID was set after re-initialization
        let ids = set_ids.lock().unwrap();
        assert!(
            ids.contains(&"session-BBB".to_string()),
            "New session ID 'session-BBB' should be set after re-initialization, got: {:?}",
            *ids
        );
    }

    /// Test 2.1a — 404 on last retry attempt still recovers (re-init doesn't count
    /// as a "retry" — the loop continues after successful re-init).
    #[tokio::test]
    async fn test_404_on_last_retry_attempt_still_recovers() {
        let mut transport = StatefulMockTransport::new();

        // Initial init
        transport.push_init_response(Ok(StatefulMockTransport::make_init_response(
            Some("session-AAA"),
            "2025-11-25",
        )));
        // 404 on first request
        transport.push_request_response(Err(McpClientError::Transport(
            crate::error::TransportError::HttpStatus {
                status: 404,
                message: "Not Found".to_string(),
            },
        )));
        // Re-init
        transport.push_init_response(Ok(StatefulMockTransport::make_init_response(
            Some("session-BBB"),
            "2025-11-25",
        )));
        // Retry succeeds
        transport.push_request_response(Ok(json!({
            "jsonrpc": "2.0",
            "id": "req_1",
            "result": { "tools": [] }
        })));

        let client = McpClient::new(Box::new(transport), fast_retry_config(2));
        client.connect().await.unwrap();

        let result = client.list_tools().await;
        assert!(
            result.is_ok(),
            "404 recovery should work even with max_attempts=2: {:?}",
            result.err()
        );
    }

    /// Test 2.1b — When re-initialization after 404 fails, the original 404 error
    /// is surfaced (not the re-init error).
    #[tokio::test]
    async fn test_404_reinit_failure_surfaces_original_error() {
        let mut transport = StatefulMockTransport::new();

        // Initial init
        transport.push_init_response(Ok(StatefulMockTransport::make_init_response(
            Some("session-AAA"),
            "2025-11-25",
        )));
        // 404
        transport.push_request_response(Err(McpClientError::Transport(
            crate::error::TransportError::HttpStatus {
                status: 404,
                message: "Session gone".to_string(),
            },
        )));
        // Re-init FAILS
        transport.push_init_response(Err(McpClientError::Transport(
            crate::error::TransportError::ConnectionFailed("Connection refused".to_string()),
        )));

        let client = McpClient::new(Box::new(transport), fast_retry_config(3));
        client.connect().await.unwrap();

        let result = client.list_tools().await;
        assert!(result.is_err());

        // Should surface the original 404 error, not the reinit ConnectionFailed error
        let err = result.unwrap_err();
        assert!(
            err.is_session_expired(),
            "Should surface original 404 error, got: {}",
            err
        );
    }

    /// Test 2.2 — Server may omit Mcp-Session-Id header (stateless mode).
    /// connect() must succeed and client must be ready.
    #[tokio::test]
    async fn test_optional_session_id_no_hard_failure() {
        let mut transport = StatefulMockTransport::new();

        // Init response WITHOUT session ID
        transport.push_init_response(Ok(StatefulMockTransport::make_init_response(
            None, // No session ID
            "2025-11-25",
        )));

        let client = McpClient::new(Box::new(transport), ClientConfig::default());
        let result = client.connect().await;
        assert!(
            result.is_ok(),
            "connect() must succeed without Mcp-Session-Id: {:?}",
            result.err()
        );
        assert!(
            client.is_ready().await,
            "Client must be ready after stateless init"
        );

        let status = client.connection_status().await;
        assert!(
            status.session_id.is_none(),
            "Session ID should be None for stateless server, got: {:?}",
            status.session_id
        );
    }

    /// Test 2.3 — Server returns an unsupported protocol version.
    /// connect() must fail with an error mentioning both versions.
    #[tokio::test]
    async fn test_unsupported_protocol_version_rejected() {
        let mut transport = StatefulMockTransport::new();

        // Server returns unsupported version
        transport.push_init_response(Ok(StatefulMockTransport::make_init_response(
            Some("session-123"),
            "2099-01-01",
        )));

        let client = McpClient::new(Box::new(transport), ClientConfig::default());
        let result = client.connect().await;
        assert!(
            result.is_err(),
            "connect() must fail for unsupported protocol version"
        );

        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("2099-01-01"),
            "Error should mention server's version '2099-01-01', got: {}",
            err_msg
        );
        assert!(
            err_msg.contains("2025-11-25"),
            "Error should mention supported version '2025-11-25', got: {}",
            err_msg
        );
    }

    // ── Error propagation tests ────────────────────────────────────────

    /// Test 4.1 — JSON-RPC error response surfaces as `ServerError` with code, message, and data.
    #[tokio::test]
    async fn test_jsonrpc_error_surfaces_as_server_error_with_code_message_data() {
        let mut transport = StatefulMockTransport::new();
        transport.push_init_response(Ok(StatefulMockTransport::make_init_response(
            Some("session-1"),
            "2025-11-25",
        )));
        transport.push_request_response(Ok(json!({
            "jsonrpc": "2.0",
            "id": "req_1",
            "error": {
                "code": -32602,
                "message": "Invalid params",
                "data": {"detail": "missing field 'name'"}
            }
        })));

        let client = McpClient::new(Box::new(transport), ClientConfig::default());
        client.connect().await.unwrap();

        let result = client.list_tools().await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.error_code(), Some(-32602));
        assert!(err.to_string().contains("Invalid params"));
        if let McpClientError::ServerError { data, .. } = &err {
            assert!(data.is_some());
            assert_eq!(data.as_ref().unwrap()["detail"], "missing field 'name'");
        } else {
            panic!("Expected ServerError, got: {:?}", err);
        }
    }

    /// Test 4.2 — JSON-RPC error without `data` field: `data` must be `None`.
    #[tokio::test]
    async fn test_jsonrpc_error_without_data_field() {
        let mut transport = StatefulMockTransport::new();
        transport.push_init_response(Ok(StatefulMockTransport::make_init_response(
            Some("session-1"),
            "2025-11-25",
        )));
        transport.push_request_response(Ok(json!({
            "jsonrpc": "2.0",
            "id": "req_1",
            "error": {
                "code": -32600,
                "message": "Invalid Request"
            }
        })));

        let client = McpClient::new(Box::new(transport), ClientConfig::default());
        client.connect().await.unwrap();

        let result = client.list_tools().await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.error_code(), Some(-32600));
        if let McpClientError::ServerError { data, .. } = &err {
            assert!(data.is_none(), "data should be None when server omits it");
        } else {
            panic!("Expected ServerError, got: {:?}", err);
        }
    }

    /// Test 4.3 — `call_tool` with a malformed response returns an error rather than panicking.
    #[tokio::test]
    async fn test_call_tool_malformed_response_returns_error() {
        let mut transport = StatefulMockTransport::new();
        transport.push_init_response(Ok(StatefulMockTransport::make_init_response(
            Some("session-1"),
            "2025-11-25",
        )));
        transport.push_request_response(Ok(json!({
            "jsonrpc": "2.0",
            "id": "req_1",
            "result": {"unexpected": "shape"}
        })));

        let client = McpClient::new(Box::new(transport), ClientConfig::default());
        client.connect().await.unwrap();

        let result = client.call_tool("test", json!({})).await;
        assert!(
            result.is_err(),
            "Malformed response should return error, not panic"
        );
    }

    /// Test 4.4 — `get_prompt` with a malformed response returns an error rather than panicking.
    #[tokio::test]
    async fn test_get_prompt_malformed_response_returns_error() {
        let mut transport = StatefulMockTransport::new();
        transport.push_init_response(Ok(StatefulMockTransport::make_init_response(
            Some("session-1"),
            "2025-11-25",
        )));
        transport.push_request_response(Ok(json!({
            "jsonrpc": "2.0",
            "id": "req_1",
            "result": {"wrong": "format"}
        })));

        let client = McpClient::new(Box::new(transport), ClientConfig::default());
        client.connect().await.unwrap();

        let result = client.get_prompt("test", None).await;
        assert!(
            result.is_err(),
            "Malformed response should return error, not panic"
        );
    }

    // ── Cache and notification tests ─────────────────────────────────────

    /// Test: list_tools() caches results and returns cached data on second call.
    #[tokio::test]
    async fn test_list_tools_caches_result() {
        let mut transport = StatefulMockTransport::new();

        // Init
        transport.push_init_response(Ok(StatefulMockTransport::make_init_response(
            Some("session-1"),
            "2025-11-25",
        )));

        // First list_tools call
        transport.push_request_response(Ok(json!({
            "jsonrpc": "2.0",
            "id": "req_1",
            "result": {
                "tools": [
                    {"name": "tool_a", "description": "Tool A", "inputSchema": {"type": "object"}}
                ]
            }
        })));

        // No second response queued — if list_tools makes a second request, it will error

        let client = McpClient::new(Box::new(transport), ClientConfig::default());
        client.connect().await.unwrap();

        let tools1 = client.list_tools().await.unwrap();
        assert_eq!(tools1.len(), 1);
        assert_eq!(tools1[0].name, "tool_a");

        // Second call should use cache (no network request)
        let tools2 = client.list_tools().await.unwrap();
        assert_eq!(tools2.len(), 1);
        assert_eq!(tools2[0].name, "tool_a");
    }

    /// Test: notifications/tools/list_changed invalidates the tool cache.
    #[tokio::test]
    async fn test_tools_list_changed_notification_invalidates_cache() {
        let (mock, _notifications) = MockTransport::new();
        let event_sender = mock.event_sender();

        let client = McpClientBuilder::new()
            .with_transport(Box::new(mock))
            .build();

        client.connect().await.unwrap();

        // Manually populate the tool cache
        {
            let mut cache = client.cached_tools.write().await;
            *cache = Some(vec![]);
        }

        // Verify cache is populated
        assert!(client.cached_tools.read().await.is_some());

        // Send notifications/tools/list_changed
        event_sender
            .send(ServerEvent::Notification(json!({
                "jsonrpc": "2.0",
                "method": "notifications/tools/list_changed"
            })))
            .unwrap();

        // Wait for the notification to be processed
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Cache should be invalidated
        assert!(
            client.cached_tools.read().await.is_none(),
            "Tool cache should be invalidated after notifications/tools/list_changed"
        );
    }

    /// Test: notifications/resources/list_changed invalidates the resource cache.
    #[tokio::test]
    async fn test_resources_list_changed_notification_invalidates_cache() {
        let (mock, _notifications) = MockTransport::new();
        let event_sender = mock.event_sender();

        let client = McpClientBuilder::new()
            .with_transport(Box::new(mock))
            .build();

        client.connect().await.unwrap();

        // Populate resource cache
        {
            let mut cache = client.cached_resources.write().await;
            *cache = Some(vec![]);
        }

        assert!(client.cached_resources.read().await.is_some());

        event_sender
            .send(ServerEvent::Notification(json!({
                "jsonrpc": "2.0",
                "method": "notifications/resources/list_changed"
            })))
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        assert!(
            client.cached_resources.read().await.is_none(),
            "Resource cache should be invalidated after notifications/resources/list_changed"
        );
    }

    /// Test: notifications/prompts/list_changed invalidates the prompt cache.
    #[tokio::test]
    async fn test_prompts_list_changed_notification_invalidates_cache() {
        let (mock, _notifications) = MockTransport::new();
        let event_sender = mock.event_sender();

        let client = McpClientBuilder::new()
            .with_transport(Box::new(mock))
            .build();

        client.connect().await.unwrap();

        // Populate prompt cache
        {
            let mut cache = client.cached_prompts.write().await;
            *cache = Some(vec![]);
        }

        assert!(client.cached_prompts.read().await.is_some());

        event_sender
            .send(ServerEvent::Notification(json!({
                "jsonrpc": "2.0",
                "method": "notifications/prompts/list_changed"
            })))
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        assert!(
            client.cached_prompts.read().await.is_none(),
            "Prompt cache should be invalidated after notifications/prompts/list_changed"
        );
    }

    /// Test: User notification callback is invoked on server notifications.
    #[tokio::test]
    async fn test_user_notification_callback_fires() {
        let (mock, _notifications) = MockTransport::new();
        let event_sender = mock.event_sender();

        let received_methods = Arc::new(parking_lot::Mutex::new(Vec::<String>::new()));
        let received_methods_clone = Arc::clone(&received_methods);

        let client = McpClientBuilder::new()
            .with_transport(Box::new(mock))
            .on_notification(move |method, _params| {
                received_methods_clone.lock().push(method.to_string());
            })
            .build();

        client.connect().await.unwrap();

        // Send a tools/list_changed notification
        event_sender
            .send(ServerEvent::Notification(json!({
                "jsonrpc": "2.0",
                "method": "notifications/tools/list_changed"
            })))
            .unwrap();

        // Send a custom notification
        event_sender
            .send(ServerEvent::Notification(json!({
                "jsonrpc": "2.0",
                "method": "notifications/custom/event",
                "params": {"key": "value"}
            })))
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let methods = received_methods.lock();
        assert!(
            methods.contains(&"notifications/tools/list_changed".to_string()),
            "User callback should receive tools/list_changed, got: {:?}",
            *methods
        );
        assert!(
            methods.contains(&"notifications/custom/event".to_string()),
            "User callback should receive custom notifications, got: {:?}",
            *methods
        );
    }

    /// Test: refresh_tools() bypasses cache.
    #[tokio::test]
    async fn test_refresh_tools_bypasses_cache() {
        let mut transport = StatefulMockTransport::new();

        transport.push_init_response(Ok(StatefulMockTransport::make_init_response(
            Some("session-1"),
            "2025-11-25",
        )));

        // First list_tools
        transport.push_request_response(Ok(json!({
            "jsonrpc": "2.0",
            "id": "req_1",
            "result": {
                "tools": [
                    {"name": "tool_a", "description": "Tool A", "inputSchema": {"type": "object"}}
                ]
            }
        })));

        // refresh_tools response (different tools)
        transport.push_request_response(Ok(json!({
            "jsonrpc": "2.0",
            "id": "req_2",
            "result": {
                "tools": [
                    {"name": "tool_a", "description": "Tool A", "inputSchema": {"type": "object"}},
                    {"name": "tool_b", "description": "Tool B", "inputSchema": {"type": "object"}}
                ]
            }
        })));

        let client = McpClient::new(Box::new(transport), ClientConfig::default());
        client.connect().await.unwrap();

        let tools1 = client.list_tools().await.unwrap();
        assert_eq!(tools1.len(), 1);

        // refresh_tools should hit the server, not cache
        let tools2 = client.refresh_tools().await.unwrap();
        assert_eq!(tools2.len(), 2);

        // Subsequent list_tools should return new cache
        let tools3 = client.list_tools().await.unwrap();
        assert_eq!(tools3.len(), 2);
    }

    // ── ReconnectableMockTransport ─────────────────────────────────────
    //
    // A mock that supports multiple connect()/disconnect() cycles by
    // advertising server_events: false (skips event listener in connect()).
    // Used for testing the -32031 session retry path.

    struct ReconnectableMockTransport {
        init_responses: Arc<std::sync::Mutex<VecDeque<McpClientResult<TransportResponse>>>>,
        request_responses: Arc<std::sync::Mutex<VecDeque<McpClientResult<Value>>>>,
        set_session_ids: Arc<std::sync::Mutex<Vec<String>>>,
        clear_count: Arc<AtomicU32>,
        connect_count: Arc<AtomicU32>,
        disconnect_count: Arc<AtomicU32>,
        connected: bool,
    }

    impl ReconnectableMockTransport {
        fn new() -> Self {
            Self {
                init_responses: Arc::new(std::sync::Mutex::new(VecDeque::new())),
                request_responses: Arc::new(std::sync::Mutex::new(VecDeque::new())),
                set_session_ids: Arc::new(std::sync::Mutex::new(Vec::new())),
                clear_count: Arc::new(AtomicU32::new(0)),
                connect_count: Arc::new(AtomicU32::new(0)),
                disconnect_count: Arc::new(AtomicU32::new(0)),
                connected: false,
            }
        }

        fn push_init_response(&mut self, resp: McpClientResult<TransportResponse>) {
            self.init_responses.lock().unwrap().push_back(resp);
        }

        fn push_request_response(&mut self, resp: McpClientResult<Value>) {
            self.request_responses.lock().unwrap().push_back(resp);
        }
    }

    #[async_trait]
    impl crate::transport::Transport for ReconnectableMockTransport {
        fn transport_type(&self) -> TransportType {
            TransportType::Http
        }

        fn capabilities(&self) -> TransportCapabilities {
            TransportCapabilities {
                streaming: false,
                bidirectional: false,
                server_events: false, // No event listener — supports multiple connect cycles
                max_message_size: None,
                persistent: false,
            }
        }

        async fn connect(&mut self) -> McpClientResult<()> {
            self.connect_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.connected = true;
            Ok(())
        }

        async fn disconnect(&mut self) -> McpClientResult<()> {
            self.disconnect_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.connected = false;
            Ok(())
        }

        fn is_connected(&self) -> bool {
            self.connected
        }

        async fn send_request(&mut self, _request: Value) -> McpClientResult<Value> {
            self.request_responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| {
                    Err(McpClientError::generic(
                        "ReconnectableMockTransport: no more request responses queued",
                    ))
                })
        }

        async fn send_request_with_headers(
            &mut self,
            _request: Value,
        ) -> McpClientResult<TransportResponse> {
            self.init_responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| {
                    Err(McpClientError::generic(
                        "ReconnectableMockTransport: no more init responses queued",
                    ))
                })
        }

        async fn send_notification(&mut self, _notification: Value) -> McpClientResult<()> {
            Ok(())
        }

        async fn send_delete(&mut self, _session_id: &str) -> McpClientResult<()> {
            Ok(())
        }

        fn set_session_id(&mut self, session_id: String) {
            self.set_session_ids.lock().unwrap().push(session_id);
        }

        fn clear_session_id(&mut self) {
            self.clear_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }

        async fn start_event_listener(&mut self) -> McpClientResult<EventReceiver> {
            // Should never be called since server_events is false
            Err(McpClientError::generic("No event listener"))
        }

        fn connection_info(&self) -> ConnectionInfo {
            ConnectionInfo {
                transport_type: TransportType::Http,
                endpoint: "reconnectable-mock://test".to_string(),
                connected: self.connected,
                capabilities: self.capabilities(),
                metadata: Value::Null,
            }
        }

        fn statistics(&self) -> TransportStatistics {
            TransportStatistics::default()
        }
    }

    // ── Session not-initialized (-32031) retry tests ──────────────────

    /// Test: -32031 error triggers disconnect + reconnect + single retry.
    /// Simulates the race condition where notifications/initialized hasn't
    /// been processed when the first tool call arrives.
    #[tokio::test]
    async fn test_session_not_initialized_triggers_reconnect_and_retry() {
        let mut transport = ReconnectableMockTransport::new();

        // 1. Initial connect → initialize succeeds with session "AAA"
        transport.push_init_response(Ok(StatefulMockTransport::make_init_response(
            Some("session-AAA"),
            "2025-11-25",
        )));

        // 2. First call_tool → server returns -32031 (initialized not processed yet)
        transport.push_request_response(Ok(json!({
            "jsonrpc": "2.0",
            "id": "req_1",
            "error": {
                "code": -32031,
                "message": "Session error: Session not initialized - client must send notifications/initialized first (strict lifecycle mode)"
            }
        })));

        // 3. Reconnect → initialize succeeds with session "BBB"
        transport.push_init_response(Ok(StatefulMockTransport::make_init_response(
            Some("session-BBB"),
            "2025-11-25",
        )));

        // 4. Retried call_tool → succeeds
        transport.push_request_response(Ok(json!({
            "jsonrpc": "2.0",
            "id": "req_2",
            "result": {
                "content": [{"type": "text", "text": "42"}],
                "isError": false
            }
        })));

        let connect_count = transport.connect_count.clone();
        let disconnect_count = transport.disconnect_count.clone();
        let set_ids = transport.set_session_ids.clone();

        let client = McpClient::new(Box::new(transport), fast_retry_config(3));
        client.connect().await.unwrap();

        // This should: fail with -32031 → disconnect → reconnect → retry → succeed
        let result = client.call_tool("add", json!({"a": 1, "b": 2})).await;
        assert!(
            result.is_ok(),
            "call_tool should succeed after -32031 reconnect: {:?}",
            result.err()
        );

        let call_result = result.unwrap();
        assert!(!call_result.is_error.unwrap_or(true));

        // Verify disconnect + reconnect happened
        // connect_count: 1 (initial) + 1 (reconnect) = 2
        assert_eq!(
            connect_count.load(Ordering::SeqCst),
            2,
            "Should have connected twice (initial + reconnect)"
        );
        assert_eq!(
            disconnect_count.load(Ordering::SeqCst),
            1,
            "Should have disconnected once during recovery"
        );

        // Verify new session ID was set
        let ids = set_ids.lock().unwrap();
        assert!(
            ids.contains(&"session-BBB".to_string()),
            "New session ID 'session-BBB' should be set after reconnect, got: {:?}",
            *ids
        );
    }

    /// Test: -32031 reconnect only retries once — if retry also fails,
    /// the retry error is returned (not the original -32031).
    #[tokio::test]
    async fn test_session_not_initialized_retry_fails_returns_retry_error() {
        let mut transport = ReconnectableMockTransport::new();

        // 1. Initial connect
        transport.push_init_response(Ok(StatefulMockTransport::make_init_response(
            Some("session-AAA"),
            "2025-11-25",
        )));

        // 2. First request → -32031
        transport.push_request_response(Ok(json!({
            "jsonrpc": "2.0",
            "id": "req_1",
            "error": {
                "code": -32031,
                "message": "Session error: Session not initialized"
            }
        })));

        // 3. Reconnect succeeds
        transport.push_init_response(Ok(StatefulMockTransport::make_init_response(
            Some("session-BBB"),
            "2025-11-25",
        )));

        // 4. Retry ALSO fails with -32031 (persistent issue)
        transport.push_request_response(Ok(json!({
            "jsonrpc": "2.0",
            "id": "req_2",
            "error": {
                "code": -32031,
                "message": "Session error: Session not initialized (still)"
            }
        })));

        let client = McpClient::new(Box::new(transport), fast_retry_config(3));
        client.connect().await.unwrap();

        let result = client.call_tool("add", json!({})).await;
        assert!(result.is_err(), "Should fail after retry also fails");

        let err = result.unwrap_err();
        assert_eq!(
            err.error_code(),
            Some(-32031),
            "Should return the retry's error"
        );
    }

    /// Test: -32031 when reconnect itself fails — original error is returned.
    #[tokio::test]
    async fn test_session_not_initialized_reconnect_fails_returns_original_error() {
        let mut transport = ReconnectableMockTransport::new();

        // 1. Initial connect
        transport.push_init_response(Ok(StatefulMockTransport::make_init_response(
            Some("session-AAA"),
            "2025-11-25",
        )));

        // 2. Request → -32031
        transport.push_request_response(Ok(json!({
            "jsonrpc": "2.0",
            "id": "req_1",
            "error": {
                "code": -32031,
                "message": "Session error: Session not initialized"
            }
        })));

        // 3. Reconnect FAILS (no init response queued → will error)

        let client = McpClient::new(Box::new(transport), fast_retry_config(3));
        client.connect().await.unwrap();

        let result = client.call_tool("add", json!({})).await;
        assert!(result.is_err(), "Should fail when reconnect fails");

        // Original -32031 error is returned
        let err = result.unwrap_err();
        assert!(
            err.is_session_not_initialized(),
            "Should return original -32031 error when reconnect fails, got: {}",
            err
        );
    }

    /// Test: "Session not initialized" message without code -32031 is still detected.
    #[tokio::test]
    async fn test_session_not_initialized_detected_by_message() {
        let mut transport = ReconnectableMockTransport::new();

        // 1. Initial connect
        transport.push_init_response(Ok(StatefulMockTransport::make_init_response(
            Some("session-AAA"),
            "2025-11-25",
        )));

        // 2. Request → error with different code but matching message
        transport.push_request_response(Ok(json!({
            "jsonrpc": "2.0",
            "id": "req_1",
            "error": {
                "code": -32000,
                "message": "Session not initialized - client must send notifications/initialized first"
            }
        })));

        // 3. Reconnect
        transport.push_init_response(Ok(StatefulMockTransport::make_init_response(
            Some("session-BBB"),
            "2025-11-25",
        )));

        // 4. Retry succeeds
        transport.push_request_response(Ok(json!({
            "jsonrpc": "2.0",
            "id": "req_2",
            "result": { "tools": [] }
        })));

        let client = McpClient::new(Box::new(transport), fast_retry_config(3));
        client.connect().await.unwrap();

        let result = client.list_tools().await;
        assert!(
            result.is_ok(),
            "Message-based detection should trigger reconnect: {:?}",
            result.err()
        );
    }

    /// Test: Non-session errors (e.g. -32602) do NOT trigger reconnect.
    #[tokio::test]
    async fn test_non_session_error_does_not_trigger_reconnect() {
        let mut transport = ReconnectableMockTransport::new();

        // 1. Initial connect
        transport.push_init_response(Ok(StatefulMockTransport::make_init_response(
            Some("session-AAA"),
            "2025-11-25",
        )));

        // 2. Request → -32602 (invalid params — not a session error)
        transport.push_request_response(Ok(json!({
            "jsonrpc": "2.0",
            "id": "req_1",
            "error": {
                "code": -32602,
                "message": "Invalid params: missing 'name'"
            }
        })));

        let connect_count = transport.connect_count.clone();

        let client = McpClient::new(Box::new(transport), fast_retry_config(1));
        client.connect().await.unwrap();

        let result = client.call_tool("add", json!({})).await;
        assert!(result.is_err());

        // Should NOT have reconnected — only the initial connect
        assert_eq!(
            connect_count.load(Ordering::SeqCst),
            1,
            "Non-session errors must not trigger reconnect"
        );
    }

    /// Test: call_tool_with_task also benefits from -32031 retry
    /// (it uses send_request_internal internally).
    #[tokio::test]
    async fn test_call_tool_with_task_also_retries_on_session_error() {
        let mut transport = ReconnectableMockTransport::new();

        // 1. Initial connect
        transport.push_init_response(Ok(StatefulMockTransport::make_init_response(
            Some("session-AAA"),
            "2025-11-25",
        )));

        // 2. call_tool_with_task → -32031
        transport.push_request_response(Ok(json!({
            "jsonrpc": "2.0",
            "id": "req_1",
            "error": {
                "code": -32031,
                "message": "Session error: Session not initialized"
            }
        })));

        // 3. Reconnect
        transport.push_init_response(Ok(StatefulMockTransport::make_init_response(
            Some("session-BBB"),
            "2025-11-25",
        )));

        // 4. Retry succeeds (sync tool result)
        transport.push_request_response(Ok(json!({
            "jsonrpc": "2.0",
            "id": "req_2",
            "result": {
                "content": [{"type": "text", "text": "done"}],
                "isError": false
            }
        })));

        let client = McpClient::new(Box::new(transport), fast_retry_config(3));
        client.connect().await.unwrap();

        let result = client
            .call_tool_with_task("slow_add", json!({"a": 1}), None)
            .await;
        assert!(
            result.is_ok(),
            "call_tool_with_task should recover from -32031: {:?}",
            result.err()
        );
    }

    /// Test: invalidate_caches() clears all caches.
    #[tokio::test]
    async fn test_invalidate_caches_clears_all() {
        let mut transport = StatefulMockTransport::new();

        transport.push_init_response(Ok(StatefulMockTransport::make_init_response(
            Some("session-1"),
            "2025-11-25",
        )));

        let client = McpClient::new(Box::new(transport), ClientConfig::default());
        client.connect().await.unwrap();

        // Populate all caches manually
        *client.cached_tools.write().await = Some(vec![]);
        *client.cached_resources.write().await = Some(vec![]);
        *client.cached_prompts.write().await = Some(vec![]);

        client.invalidate_caches().await;

        assert!(client.cached_tools.read().await.is_none());
        assert!(client.cached_resources.read().await.is_none());
        assert!(client.cached_prompts.read().await.is_none());
    }
}
