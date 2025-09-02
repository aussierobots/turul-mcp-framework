//! Lambda MCP handler that processes requests directly with JsonRpcDispatcher
//!
//! This module provides the core LambdaMcpHandler that processes Lambda HTTP
//! requests using the turul-mcp-framework components without SessionMcpHandler.

use std::collections::HashMap;
use std::sync::Arc;

use lambda_http::{Body as LambdaBody, Request as LambdaRequest, Response as LambdaResponse};
use tracing::{debug, error, info};
use uuid::Uuid;

use turul_http_mcp_server::{
    ServerConfig, SharedNotificationBroadcaster, StreamManager,
    StreamManagerNotificationBroadcaster,
};
use turul_mcp_json_rpc_server::{
    JsonRpcDispatcher,
    r#async::SessionContext as JsonRpcSessionContext,
    dispatch::{JsonRpcMessage, JsonRpcMessageResult, parse_json_rpc_message},
};
use turul_mcp_protocol::ServerCapabilities;
use turul_mcp_session_storage::BoxedSessionStorage;

use crate::adapter::{extract_mcp_headers, inject_mcp_headers};
use crate::error::{LambdaError, Result};

#[cfg(feature = "cors")]
use crate::cors::{CorsConfig, create_preflight_response, inject_cors_headers};

/// Main handler for Lambda MCP requests
///
/// This handler processes MCP requests in Lambda by:
///
/// 1. Extracting JSON-RPC from Lambda requests
/// 2. Managing sessions with pluggable storage
/// 3. Processing through JsonRpcDispatcher directly
/// 4. Handling SSE streaming and CORS for Lambda
#[derive(Clone)]
pub struct LambdaMcpHandler {
    /// JSON-RPC dispatcher for method handling
    dispatcher: Arc<JsonRpcDispatcher>,

    /// Session storage backend
    session_storage: Arc<BoxedSessionStorage>,

    /// Stream manager for SSE notifications
    stream_manager: Arc<StreamManager>,

    /// Server configuration (currently unused - stored for future extensibility)
    #[allow(dead_code)]
    config: ServerConfig,

    /// Server implementation info (for initialize response)
    implementation: turul_mcp_protocol::Implementation,

    /// Server capabilities (for initialize response)
    capabilities: ServerCapabilities,

    /// CORS configuration (if enabled)
    #[cfg(feature = "cors")]
    cors_config: Option<CorsConfig>,
}

impl LambdaMcpHandler {
    /// Create a new Lambda MCP handler with the framework components
    pub fn new(
        dispatcher: JsonRpcDispatcher,
        session_storage: Arc<BoxedSessionStorage>,
        stream_manager: Arc<StreamManager>,
        config: ServerConfig,
        implementation: turul_mcp_protocol::Implementation,
        capabilities: ServerCapabilities,
        #[cfg(feature = "cors")] cors_config: Option<CorsConfig>,
    ) -> Self {
        Self {
            dispatcher: Arc::new(dispatcher),
            session_storage,
            stream_manager,
            config,
            implementation,
            capabilities,
            #[cfg(feature = "cors")]
            cors_config,
        }
    }

    /// Create with shared stream manager (for advanced use cases)
    pub fn with_shared_stream_manager(
        config: ServerConfig,
        dispatcher: Arc<JsonRpcDispatcher>,
        session_storage: Arc<BoxedSessionStorage>,
        stream_manager: Arc<StreamManager>,
        implementation: turul_mcp_protocol::Implementation,
        capabilities: ServerCapabilities,
    ) -> Self {
        Self {
            dispatcher,
            session_storage,
            stream_manager,
            config,
            implementation,
            capabilities,
            #[cfg(feature = "cors")]
            cors_config: None,
        }
    }

    /// Set CORS configuration
    #[cfg(feature = "cors")]
    pub fn with_cors(mut self, cors_config: CorsConfig) -> Self {
        self.cors_config = Some(cors_config);
        self
    }

    /// Get access to the underlying stream manager for notifications
    pub fn stream_manager(&self) -> &Arc<StreamManager> {
        &self.stream_manager
    }

    /// Handle a Lambda HTTP request
    ///
    /// This is the main entry point that processes all Lambda requests
    /// and returns appropriate responses.
    pub async fn handle(&self, req: LambdaRequest) -> Result<LambdaResponse<LambdaBody>> {
        let method = req.method().clone();
        let uri = req.uri().clone();
        let request_origin = req
            .headers()
            .get("origin")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        info!(
            "🌐 Lambda MCP request: {} {} (origin: {:?})",
            method, uri, request_origin
        );

        // Handle CORS preflight requests first
        #[cfg(feature = "cors")]
        if method == http::Method::OPTIONS {
            if let Some(ref cors_config) = self.cors_config {
                debug!("Handling CORS preflight request");
                return create_preflight_response(cors_config, request_origin.as_deref())
                    .map_err(Into::into);
            }
        }

        // Extract MCP-specific headers
        let mcp_headers = extract_mcp_headers(&req);
        debug!("Extracted MCP headers: {:?}", mcp_headers);

        // Handle GET requests for SSE streaming
        if method == http::Method::GET {
            return self
                .handle_sse_request(req, mcp_headers, request_origin.as_deref())
                .await;
        }

        // Handle POST requests for JSON-RPC
        if method == http::Method::POST {
            return self
                .handle_jsonrpc_request(req, mcp_headers, request_origin.as_deref())
                .await;
        }

        // Unsupported method
        let mut lambda_resp = LambdaResponse::builder()
            .status(405)
            .body(LambdaBody::Text("Method Not Allowed".to_string()))
            .map_err(LambdaError::Http)?;

        // Inject MCP headers into the response
        inject_mcp_headers(&mut lambda_resp, mcp_headers);

        // Apply CORS headers if configured
        #[cfg(feature = "cors")]
        if let Some(ref cors_config) = self.cors_config {
            inject_cors_headers(&mut lambda_resp, cors_config, request_origin.as_deref())?;
        }

        Ok(lambda_resp)
    }

    /// Handle POST JSON-RPC requests
    async fn handle_jsonrpc_request(
        &self,
        req: LambdaRequest,
        mcp_headers: HashMap<String, String>,
        request_origin: Option<&str>,
    ) -> Result<LambdaResponse<LambdaBody>> {
        // Extract request body
        let body_bytes = match req.body() {
            LambdaBody::Empty => Vec::new(),
            LambdaBody::Text(text) => text.as_bytes().to_vec(),
            LambdaBody::Binary(bytes) => bytes.clone(),
        };

        // Parse JSON-RPC request (exact pattern from SessionMcpHandler)
        let body_str = String::from_utf8(body_bytes)
            .map_err(|e| LambdaError::Body(format!("Invalid UTF-8: {}", e)))?;

        debug!("🔍 RAW JSON REQUEST: {}", body_str);
        debug!("📊 RAW REQUEST HEADERS: {:?}", mcp_headers);
        debug!("🌐 HTTP METHOD: POST (JSON-RPC)");
        debug!("📍 REQUEST URI: {}", req.uri());

        // Try to parse as JSON first to see the structure
        match serde_json::from_str::<serde_json::Value>(&body_str) {
            Ok(json_value) => {
                debug!(
                    "📝 PARSED JSON STRUCTURE: {}",
                    serde_json::to_string_pretty(&json_value)
                        .unwrap_or_else(|_| "failed to pretty print".to_string())
                );
            }
            Err(json_err) => {
                error!("❌ INVALID JSON: {}", json_err);
            }
        }

        // Parse JSON-RPC message using framework parser
        let message = match parse_json_rpc_message(&body_str) {
            Ok(msg) => {
                debug!("✅ PARSED JSON-RPC MESSAGE: {:?}", msg);
                msg
            }
            Err(rpc_err) => {
                error!("❌ JSON-RPC PARSE ERROR: {}", rpc_err);
                error!("❌ FAILED ON BODY: {}", body_str);
                let error_response =
                    serde_json::to_string(&rpc_err).unwrap_or_else(|_| "{}".to_string());

                error!("🔄 ERROR RESPONSE JSON: {}", error_response);

                let mut lambda_resp = LambdaResponse::builder()
                    .status(200) // JSON-RPC errors still use 200 OK
                    .header("content-type", "application/json")
                    .body(LambdaBody::Text(error_response))
                    .map_err(LambdaError::Http)?;

                inject_mcp_headers(&mut lambda_resp, mcp_headers);

                #[cfg(feature = "cors")]
                if let Some(ref cors_config) = self.cors_config {
                    inject_cors_headers(&mut lambda_resp, cors_config, request_origin)?;
                }

                return Ok(lambda_resp);
            }
        };

        // Extract session ID from headers
        let session_id = mcp_headers.get("mcp-session-id").cloned();

        // Handle the message using proper JSON-RPC enums (exact pattern from SessionMcpHandler)
        let (message_result, response_session_id, _method_name) = match message {
            JsonRpcMessage::Request(request) => {
                debug!("Processing JSON-RPC request: method={}", request.method);

                // Special handling for initialize requests - they create new sessions
                let (response, response_session_id) = if request.method == "initialize" {
                    debug!(
                        "Handling initialize request - creating new session via session storage"
                    );

                    // Let session storage create the session and generate the ID
                    match self
                        .session_storage
                        .create_session(self.capabilities.clone())
                        .await
                    {
                        Ok(session_info) => {
                            debug!(
                                "Created new session via session storage: {}",
                                session_info.session_id
                            );

                            // Create session-specific notification broadcaster from shared StreamManager
                            let broadcaster: SharedNotificationBroadcaster =
                                Arc::new(StreamManagerNotificationBroadcaster::new(Arc::clone(
                                    &self.stream_manager,
                                )));
                            let broadcaster_any =
                                Arc::new(broadcaster) as Arc<dyn std::any::Any + Send + Sync>;

                            let _session_context = JsonRpcSessionContext {
                                session_id: session_info.session_id.clone(),
                                metadata: HashMap::new(),
                                broadcaster: Some(broadcaster_any),
                                timestamp: chrono::Utc::now().timestamp_millis() as u64,
                            };

                            // Handle initialize request with proper MCP protocol response
                            use turul_mcp_protocol::{InitializeResult, version::McpVersion};
                            let initialize_result = InitializeResult::new(
                                McpVersion::V2025_06_18,
                                self.capabilities.clone(),
                                self.implementation.clone(),
                            );

                            let response_value = match serde_json::to_value(initialize_result) {
                                Ok(value) => value,
                                Err(e) => {
                                    error!("Failed to serialize InitializeResult: {}", e);
                                    serde_json::json!({"error": "Failed to serialize initialize result"})
                                }
                            };

                            let response = turul_mcp_json_rpc_server::JsonRpcResponse::success(
                                request.id,
                                response_value,
                            );

                            // Return the session ID created by session storage for the HTTP header
                            (response, Some(session_info.session_id))
                        }
                        Err(err) => {
                            error!("Failed to create session during initialize: {}", err);
                            let error_msg = format!("Session creation failed: {}", err);
                            let error_response =
                                turul_mcp_json_rpc_server::JsonRpcResponse::success(
                                    request.id,
                                    serde_json::json!({"error": error_msg}),
                                );
                            (error_response, None)
                        }
                    }
                } else {
                    // Use shared StreamManager for notification broadcaster
                    let session_id_str = session_id.clone().unwrap_or("unknown".to_string());
                    let broadcaster: SharedNotificationBroadcaster = Arc::new(
                        StreamManagerNotificationBroadcaster::new(Arc::clone(&self.stream_manager)),
                    );
                    let broadcaster_any =
                        Arc::new(broadcaster) as Arc<dyn std::any::Any + Send + Sync>;

                    let session_context = JsonRpcSessionContext {
                        session_id: session_id_str,
                        metadata: HashMap::new(),
                        broadcaster: Some(broadcaster_any),
                        timestamp: chrono::Utc::now().timestamp_millis() as u64,
                    };

                    let response = self
                        .dispatcher
                        .handle_request_with_context(request, session_context)
                        .await;
                    (response, session_id)
                };

                (
                    JsonRpcMessageResult::Response(response),
                    response_session_id,
                    None::<String>,
                )
            }
            JsonRpcMessage::Notification(notification) => {
                debug!(
                    "Processing JSON-RPC notification: method={}",
                    notification.method
                );

                // Create session context with shared StreamManager broadcaster
                let session_context = if let Some(ref session_id) = session_id {
                    let broadcaster: SharedNotificationBroadcaster = Arc::new(
                        StreamManagerNotificationBroadcaster::new(Arc::clone(&self.stream_manager)),
                    );
                    let broadcaster_any =
                        Arc::new(broadcaster) as Arc<dyn std::any::Any + Send + Sync>;

                    Some(JsonRpcSessionContext {
                        session_id: session_id.clone(),
                        metadata: HashMap::new(),
                        broadcaster: Some(broadcaster_any),
                        timestamp: chrono::Utc::now().timestamp_millis() as u64,
                    })
                } else {
                    None
                };

                if let Err(err) = self
                    .dispatcher
                    .handle_notification_with_context(notification, session_context)
                    .await
                {
                    error!("Notification handling error: {}", err);
                }
                (JsonRpcMessageResult::NoResponse, session_id.clone(), None)
            }
        };

        // Convert message result to Lambda response
        let response_json = match message_result {
            JsonRpcMessageResult::Response(response) => serde_json::to_value(response)
                .map_err(|e| LambdaError::Body(format!("Failed to serialize response: {}", e)))?,
            JsonRpcMessageResult::Error(error) => {
                // Handle JSON-RPC error
                serde_json::to_value(error)
                    .map_err(|e| LambdaError::Body(format!("Failed to serialize error: {}", e)))?
            }
            JsonRpcMessageResult::NoResponse => {
                // For notifications, return 202 Accepted
                let mut lambda_resp = LambdaResponse::builder()
                    .status(202)
                    .body(LambdaBody::Empty)
                    .map_err(LambdaError::Http)?;

                inject_mcp_headers(&mut lambda_resp, mcp_headers);

                if let Some(ref session_id) = response_session_id {
                    lambda_resp
                        .headers_mut()
                        .insert("mcp-session-id", session_id.parse().unwrap());
                }

                #[cfg(feature = "cors")]
                if let Some(ref cors_config) = self.cors_config {
                    inject_cors_headers(&mut lambda_resp, cors_config, request_origin)?;
                }

                return Ok(lambda_resp);
            }
        };

        // Create Lambda response
        let response_body = serde_json::to_string(&response_json)
            .map_err(|e| LambdaError::Body(format!("Failed to serialize response: {}", e)))?;

        let response_body_len = response_body.len();

        debug!("🔄 RESPONSE JSON: {}", response_body);
        debug!(
            "📝 RESPONSE JSON STRUCTURE: {}",
            serde_json::to_string_pretty(&response_json)
                .unwrap_or_else(|_| "failed to pretty print".to_string())
        );
        debug!("📏 RESPONSE SIZE: {} bytes", response_body_len);

        let mut lambda_resp = LambdaResponse::builder()
            .status(200)
            .header("content-type", "application/json")
            .body(LambdaBody::Text(response_body))
            .map_err(LambdaError::Http)?;

        // Add session ID header if we have one
        if let Some(ref session_id) = response_session_id {
            lambda_resp
                .headers_mut()
                .insert("mcp-session-id", session_id.parse().unwrap());
        }

        // Inject MCP headers
        inject_mcp_headers(&mut lambda_resp, mcp_headers);

        // Apply CORS headers if configured
        #[cfg(feature = "cors")]
        if let Some(ref cors_config) = self.cors_config {
            inject_cors_headers(&mut lambda_resp, cors_config, request_origin.as_deref())?;
        }

        debug!(
            "📤 FINAL HTTP RESPONSE HEADERS: {:?}",
            lambda_resp.headers()
        );
        debug!("📊 FINAL HTTP STATUS: {}", lambda_resp.status());
        info!("✅ Lambda JSON-RPC response: {} bytes", response_body_len);
        Ok(lambda_resp)
    }

    /// Handle GET SSE requests
    async fn handle_sse_request(
        &self,
        _req: LambdaRequest,
        mcp_headers: HashMap<String, String>,
        request_origin: Option<&str>,
    ) -> Result<LambdaResponse<LambdaBody>> {
        // Extract session ID (required for SSE)
        let session_id: Uuid = mcp_headers
            .get("mcp-session-id")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| LambdaError::Session("Missing session ID for SSE".to_string()))?;

        // Verify session exists in storage
        match self
            .session_storage
            .get_session(&session_id.to_string())
            .await
        {
            Ok(Some(_)) => {}
            Ok(None) => return Err(LambdaError::Session("Session not found".to_string())),
            Err(e) => return Err(LambdaError::Session(e.to_string())),
        }

        #[cfg(feature = "sse")]
        {
            // For now, return a simple SSE response with a heartbeat
            // TODO: Implement proper SSE streaming when the framework API is available
            let sse_content = format!(
                "data: {{\"type\":\"heartbeat\",\"timestamp\":{}}}\n\n",
                chrono::Utc::now().timestamp_millis()
            );

            let mut lambda_resp = LambdaResponse::builder()
                .status(200)
                .header("content-type", "text/event-stream")
                .header("cache-control", "no-cache")
                .header("connection", "keep-alive")
                .header("mcp-session-id", session_id.to_string())
                .body(LambdaBody::Text(sse_content))
                .map_err(LambdaError::Http)?;

            // Apply CORS headers if configured
            #[cfg(feature = "cors")]
            if let Some(ref cors_config) = self.cors_config {
                inject_cors_headers(&mut lambda_resp, cors_config, request_origin.as_deref())?;
            }

            info!("✅ Lambda SSE stream for session: {}", session_id);
            Ok(lambda_resp)
        }

        #[cfg(not(feature = "sse"))]
        {
            Err(LambdaError::Config("SSE feature not enabled".to_string()))
        }
    }

    // Session management is now handled by the framework during initialize requests
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use turul_http_mcp_server::{JsonRpcHandler, ServerConfig, StreamConfig, StreamManager};
    use turul_mcp_json_rpc_server::JsonRpcDispatcher;
    use turul_mcp_protocol::{Implementation, ServerCapabilities};
    use turul_mcp_session_storage::InMemorySessionStorage;

    #[tokio::test]
    async fn test_handler_creation() {
        // Test basic handler components
        let dispatcher = Arc::new(JsonRpcDispatcher::new());
        let session_storage = Arc::new(InMemorySessionStorage::new());
        let stream_manager = Arc::new(StreamManager::with_config(
            session_storage.clone(),
            StreamConfig::default(),
        ));
        let config = ServerConfig::default();
        let implementation = Implementation {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            title: Some("Test Server".to_string()),
        };
        let capabilities = ServerCapabilities::default();

        let handler = LambdaMcpHandler::new(
            Arc::try_unwrap(dispatcher).unwrap_or_else(|_arc| {
                // Fallback if Arc has multiple references - create a new dispatcher
                JsonRpcDispatcher::new()
            }),
            session_storage,
            stream_manager,
            config,
            implementation,
            capabilities,
            #[cfg(feature = "cors")]
            None,
        );

        // Test that handler was created successfully
        assert!(handler.stream_manager().as_ref() as *const _ != std::ptr::null());
    }

    #[tokio::test]
    async fn test_bridge_creation() {
        use turul_mcp_server::handlers::PingHandler;
        use turul_mcp_server::{SessionAwareMcpHandlerBridge, SessionManager};

        let ping_handler = Arc::new(PingHandler);
        let session_storage = Arc::new(InMemorySessionStorage::new());
        let session_manager = Arc::new(SessionManager::with_storage_and_timeouts(
            session_storage,
            ServerCapabilities::default(),
            std::time::Duration::from_secs(30 * 60),
            std::time::Duration::from_secs(60),
        ));
        let bridge = SessionAwareMcpHandlerBridge::new(ping_handler, session_manager);

        assert_eq!(bridge.supported_methods(), vec!["ping".to_string()]);
    }
}
