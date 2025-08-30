//! JSON-RPC 2.0 over HTTP handler for MCP requests with SessionStorage integration
//!
//! This handler implements proper JSON-RPC 2.0 server over HTTP transport with
//! MCP 2025-06-18 compliance, including:
//! - SessionStorage trait integration (defaults to InMemory)
//! - StreamManager for SSE with resumability
//! - 202 Accepted for notifications
//! - Last-Event-ID header support
//! - Per-session event targeting

use std::sync::Arc;
use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};
use tracing::{debug, warn, error};

use hyper::{Request, Response, Method, StatusCode};
use bytes::Bytes;
use hyper::header::{CONTENT_TYPE, ACCEPT};
use http_body_util::{BodyExt, Full};
use http_body::{Body, Frame};
use futures::Stream;

use turul_mcp_json_rpc_server::{
    JsonRpcDispatcher,
    r#async::SessionContext,
    dispatch::{parse_json_rpc_message, JsonRpcMessage, JsonRpcMessageResult},
    error::{JsonRpcError, JsonRpcErrorObject}
};
use turul_mcp_session_storage::{SessionStorage, InMemorySessionStorage};
use turul_mcp_protocol::ServerCapabilities;
use chrono;
use uuid::Uuid;

use crate::{
    Result, ServerConfig, StreamConfig,
    protocol::{extract_protocol_version, extract_session_id, extract_last_event_id},
    json_rpc_responses::*,
    StreamManager,
    notification_bridge::{StreamManagerNotificationBroadcaster, SharedNotificationBroadcaster}
};

/// SSE stream body that implements hyper's Body trait
pub struct SessionSseStream {
    stream: Pin<Box<dyn Stream<Item = std::result::Result<Bytes, Infallible>> + Send>>,
}

impl SessionSseStream {
    pub fn new<S>(stream: S) -> Self
    where
        S: Stream<Item = std::result::Result<Bytes, Infallible>> + Send + 'static,
    {
        Self {
            stream: Box::pin(stream),
        }
    }
}

impl Drop for SessionSseStream {
    fn drop(&mut self) {
        debug!("🔥 DROP: SessionSseStream - HTTP response body being cleaned up");
        debug!("🔥 This may indicate early cleanup of SSE response stream");
    }
}

impl Body for SessionSseStream {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<std::result::Result<Frame<Self::Data>, Self::Error>>> {
        match self.stream.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(data))) => {
                Poll::Ready(Some(Ok(Frame::data(data))))
            }
            Poll::Ready(Some(Err(never))) => match never {},
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// HTTP body type for JSON-RPC responses
type JsonRpcBody = Full<Bytes>;

/// HTTP body type for unified MCP responses (can handle both JSON-RPC and streaming)
type UnifiedMcpBody = http_body_util::combinators::UnsyncBoxBody<Bytes, hyper::Error>;

/// Helper function to convert Full<Bytes> to UnsyncBoxBody<Bytes, hyper::Error>
fn convert_to_unified_body(full_body: Full<Bytes>) -> UnifiedMcpBody {
    full_body.map_err(|never| match never {}).boxed_unsync()
}

/// Helper function to create JSON-RPC error response as unified body
fn jsonrpc_error_to_unified_body(error: JsonRpcError) -> Result<Response<UnifiedMcpBody>> {
    let error_json = serde_json::to_string(&error)?;
    Ok(Response::builder()
        .status(StatusCode::OK) // JSON-RPC errors still use 200 OK
        .header(CONTENT_TYPE, "application/json")
        .body(convert_to_unified_body(Full::new(Bytes::from(error_json))))
        .unwrap())
}

// ✅ CORRECTED ARCHITECTURE: Remove complex registry - use single shared StreamManager

/// JSON-RPC 2.0 over HTTP handler with shared StreamManager
pub struct SessionMcpHandler<S: SessionStorage = InMemorySessionStorage> {
    pub(crate) config: ServerConfig,
    pub(crate) dispatcher: Arc<JsonRpcDispatcher>,
    pub(crate) session_storage: Arc<S>,
    pub(crate) stream_config: StreamConfig,
    // ✅ CORRECTED ARCHITECTURE: Single shared StreamManager instance with internal session management
    pub(crate) stream_manager: Arc<StreamManager<S>>,
}

impl<S: SessionStorage + 'static> Clone for SessionMcpHandler<S> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            dispatcher: Arc::clone(&self.dispatcher),
            session_storage: Arc::clone(&self.session_storage),
            stream_config: self.stream_config.clone(),
            stream_manager: Arc::clone(&self.stream_manager),
        }
    }
}

impl SessionMcpHandler<InMemorySessionStorage> {
    /// Create a new handler with default in-memory storage (zero-configuration)
    pub fn new(
        config: ServerConfig,
        dispatcher: Arc<JsonRpcDispatcher>,
        stream_config: StreamConfig,
    ) -> Self {
        let storage = Arc::new(InMemorySessionStorage::new());
        Self::with_storage(config, dispatcher, storage, stream_config)
    }
}

impl<S: SessionStorage + 'static> SessionMcpHandler<S> {
    /// Create handler with shared StreamManager instance (corrected architecture)
    pub fn with_shared_stream_manager(
        config: ServerConfig,
        dispatcher: Arc<JsonRpcDispatcher>,
        session_storage: Arc<S>,
        stream_config: StreamConfig,
        stream_manager: Arc<StreamManager<S>>,
    ) -> Self {
        Self {
            config,
            dispatcher,
            session_storage,
            stream_config,
            stream_manager,
        }
    }

    /// Create handler with specific session storage backend (creates own StreamManager)
    /// Note: Use with_shared_stream_manager for correct architecture
    pub fn with_storage(
        config: ServerConfig,
        dispatcher: Arc<JsonRpcDispatcher>,
        session_storage: Arc<S>,
        stream_config: StreamConfig,
    ) -> Self {
        // Create own StreamManager instance (not recommended for production)
        let stream_manager = Arc::new(StreamManager::with_config(
            Arc::clone(&session_storage),
            stream_config.clone()
        ));

        Self {
            config,
            dispatcher,
            session_storage,
            stream_config,
            stream_manager,
        }
    }

    /// Get access to the StreamManager for notifications
    pub fn get_stream_manager(&self) -> &Arc<StreamManager<S>> {
        &self.stream_manager
    }


    /// Handle MCP HTTP requests with full MCP 2025-06-18 compliance
    pub async fn handle_mcp_request(
        &self,
        req: Request<hyper::body::Incoming>,
    ) -> Result<Response<UnifiedMcpBody>> {
        match req.method() {
            &Method::POST => {
                let response = self.handle_json_rpc_request(req).await?;
                Ok(response.map(convert_to_unified_body))
            },
            &Method::GET => self.handle_sse_request(req).await,
            &Method::DELETE => {
                let response = self.handle_delete_request(req).await?;
                Ok(response.map(convert_to_unified_body))
            },
            &Method::OPTIONS => {
                let response = self.handle_preflight();
                Ok(response.map(convert_to_unified_body))
            },
            _ => {
                let response = self.method_not_allowed();
                Ok(response.map(convert_to_unified_body))
            }
        }
    }

    /// Handle JSON-RPC requests over HTTP POST
    async fn handle_json_rpc_request(
        &self,
        req: Request<hyper::body::Incoming>,
    ) -> Result<Response<Full<Bytes>>> {
        // Extract protocol version and session ID from headers
        let protocol_version = extract_protocol_version(req.headers());
        let session_id = extract_session_id(req.headers());

        debug!("POST request - Protocol: {}, Session: {:?}", protocol_version, session_id);

        // Check content type
        let content_type = req.headers()
            .get(CONTENT_TYPE)
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or("");

        if !content_type.starts_with("application/json") {
            warn!("Invalid content type: {}", content_type);
            return Ok(bad_request_response("Content-Type must be application/json"));
        }

        // Check if client accepts SSE for streaming responses (MCP Streamable HTTP)
        let accept_header = req.headers()
            .get(ACCEPT)
            .and_then(|accept| accept.to_str().ok())
            .unwrap_or("application/json");

        let accepts_sse = accept_header.contains("text/event-stream");
        debug!("POST request Accept header: '{}', will use SSE for tool calls: {}", accept_header, accepts_sse);

        // Read request body
        let body = req.into_body();
        let body_bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(err) => {
                error!("Failed to read request body: {}", err);
                return Ok(bad_request_response("Failed to read request body"));
            }
        };

        // Check body size
        if body_bytes.len() > self.config.max_body_size {
            warn!("Request body too large: {} bytes", body_bytes.len());
            return Ok(Response::builder()
                .status(StatusCode::PAYLOAD_TOO_LARGE)
                .header(CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from("Request body too large")))
                .unwrap());
        }

        // Parse as UTF-8
        let body_str = match std::str::from_utf8(&body_bytes) {
            Ok(s) => s,
            Err(err) => {
                error!("Invalid UTF-8 in request body: {}", err);
                return Ok(bad_request_response("Request body must be valid UTF-8"));
            }
        };

        debug!("Received JSON-RPC request: {}", body_str);

        // Parse JSON-RPC message
        let message = match parse_json_rpc_message(body_str) {
            Ok(msg) => msg,
            Err(rpc_err) => {
                error!("JSON-RPC parse error: {}", rpc_err);
                // Extract request ID from the error if available
                let error_response = serde_json::to_string(&rpc_err)
                    .unwrap_or_else(|_| "{}".to_string());
                return Ok(Response::builder()
                    .status(StatusCode::OK) // JSON-RPC parse errors still use 200 OK
                    .header(CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(error_response)))
                    .unwrap());
            }
        };

        // Handle the message using proper JSON-RPC enums
        let (message_result, response_session_id, method_name) = match message {
            JsonRpcMessage::Request(request) => {
                debug!("Processing JSON-RPC request: method={}", request.method);
                let method_name = request.method.clone();

                // Special handling for initialize requests - they create new sessions
                let (response, response_session_id) = if request.method == "initialize" {
                    debug!("Handling initialize request - creating new session via session storage");

                    // Let session storage create the session and generate the ID (GPS pattern)
                    let capabilities = ServerCapabilities::default();
                    match self.session_storage.create_session(capabilities).await {
                        Ok(session_info) => {
                            debug!("Created new session via session storage: {}", session_info.session_id);

                            // ✅ CORRECTED ARCHITECTURE: Create session-specific notification broadcaster from shared StreamManager
                            let broadcaster: SharedNotificationBroadcaster = Arc::new(StreamManagerNotificationBroadcaster::new(
                                Arc::clone(&self.stream_manager)
                            ));
                            let broadcaster_any = Arc::new(broadcaster) as Arc<dyn std::any::Any + Send + Sync>;

                            let session_context = SessionContext {
                                session_id: session_info.session_id.clone(),
                                metadata: std::collections::HashMap::new(),
                                broadcaster: Some(broadcaster_any),
                                timestamp: chrono::Utc::now().timestamp_millis() as u64,
                            };

                            let response = self.dispatcher.handle_request_with_context(request, session_context).await;

                            // Return the session ID created by session storage for the HTTP header
                            (response, Some(session_info.session_id))
                        }
                        Err(err) => {
                            error!("Failed to create session during initialize: {}", err);
                            // Return error response - this will be converted to proper error response by dispatcher
                            let error_msg = format!("Session creation failed: {}", err);
                            let error_response = turul_mcp_json_rpc_server::JsonRpcResponse::success(
                                request.id,
                                serde_json::json!({"error": error_msg})
                            );
                            (error_response, None)
                        }
                    }
                } else {
                    // ✅ CORRECTED ARCHITECTURE: Use shared StreamManager for notification broadcaster
                    let session_id_str = session_id.clone().unwrap_or("unknown".to_string());
                    let broadcaster: SharedNotificationBroadcaster = Arc::new(StreamManagerNotificationBroadcaster::new(
                        Arc::clone(&self.stream_manager)
                    ));
                    let broadcaster_any = Arc::new(broadcaster) as Arc<dyn std::any::Any + Send + Sync>;

                    let session_context = SessionContext {
                        session_id: session_id_str,
                        metadata: std::collections::HashMap::new(),
                        broadcaster: Some(broadcaster_any),
                        timestamp: chrono::Utc::now().timestamp_millis() as u64,
                    };

                    let response = self.dispatcher.handle_request_with_context(request, session_context).await;
                    (response, session_id)
                };

                (JsonRpcMessageResult::Response(response), response_session_id, Some(method_name))
            }
            JsonRpcMessage::Notification(notification) => {
                debug!("Processing JSON-RPC notification: method={}", notification.method);
                let method_name = notification.method.clone();

                // ✅ CORRECTED ARCHITECTURE: Create session context with shared StreamManager broadcaster
                let session_context = if let Some(ref session_id) = session_id {
                    let broadcaster: SharedNotificationBroadcaster = Arc::new(StreamManagerNotificationBroadcaster::new(
                        Arc::clone(&self.stream_manager)
                    ));
                    let broadcaster_any = Arc::new(broadcaster) as Arc<dyn std::any::Any + Send + Sync>;

                    Some(SessionContext {
                        session_id: session_id.clone(),
                        metadata: std::collections::HashMap::new(),
                        broadcaster: Some(broadcaster_any),
                        timestamp: chrono::Utc::now().timestamp_millis() as u64,
                    })
                } else {
                    None
                };

                if let Err(err) = self.dispatcher.handle_notification_with_context(notification, session_context).await {
                    error!("Notification handling error: {}", err);
                }
                (JsonRpcMessageResult::NoResponse, session_id.clone(), Some(method_name))
            }
        };

        // Convert message result to HTTP response
        match message_result {
            JsonRpcMessageResult::Response(response) => {
                // Check if this is a tool call that should return SSE
                // Only use SSE if explicitly requested via Accept: text/event-stream header
                let is_tool_call = method_name.as_ref().map_or(false, |m| m == "tools/call");

                debug!("Decision point: method={:?}, accepts_sse={}, session_id={:?}, is_tool_call={}",
                       method_name, accepts_sse, response_session_id, is_tool_call);

                // TEMPORARY FIX: Disable MCP Streamable HTTP for tool calls to ensure MCP Inspector compatibility
                // Always return JSON responses for all operations until SSE implementation is fixed
                debug!("🔧 COMPATIBILITY MODE: Always returning JSON response for method: {:?} (SSE disabled for tool calls)", method_name);
                Ok(jsonrpc_response_with_session(response, response_session_id)?)
            }
            JsonRpcMessageResult::Error(error) => {
                warn!("Sending JSON-RPC error response");
                // Convert JsonRpcError to proper HTTP response
                let error_json = serde_json::to_string(&error)?;
                Ok(Response::builder()
                    .status(StatusCode::OK) // JSON-RPC errors still return 200 OK
                    .header(CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(error_json)))
                    .unwrap())
            }
            JsonRpcMessageResult::NoResponse => {
                // Notifications don't return responses (204 No Content)
                Ok(jsonrpc_notification_response()?)
            }
        }
    }

    // Note: create_post_sse_response method removed as it's unused in MCP Inspector compatibility mode
    // SSE for tool calls is temporarily disabled - see WORKING_MEMORY.md for details

    /// Handle Server-Sent Events requests (SSE for streaming)
    async fn handle_sse_request(
        &self,
        req: Request<hyper::body::Incoming>,
    ) -> Result<Response<UnifiedMcpBody>> {
        // Check if client accepts SSE
        let headers = req.headers();
        let accept = headers
            .get(ACCEPT)
            .and_then(|accept| accept.to_str().ok())
            .unwrap_or("");

        if !accept.contains("text/event-stream") {
            warn!("GET request received without SSE support - header does not contain 'text/event-stream'");
            let error = JsonRpcError::new(
                None,
                JsonRpcErrorObject::server_error(
                    -32001,
                    "SSE not accepted - missing 'text/event-stream' in Accept header",
                    None
                )
            );
            return jsonrpc_error_to_unified_body(error);
        }

        // Extract protocol version and session ID
        let protocol_version = extract_protocol_version(headers);
        let session_id = extract_session_id(headers);

        debug!("GET SSE request - Protocol: {}, Session: {:?}", protocol_version, session_id);

        // Session ID is required for SSE
        let session_id = match session_id {
            Some(id) => id,
            None => {
                warn!("Missing Mcp-Session-Id header for SSE request");
                let error = JsonRpcError::new(
                    None,
                    JsonRpcErrorObject::server_error(
                        -32002,
                        "Missing Mcp-Session-Id header",
                        None
                    )
                );
                return jsonrpc_error_to_unified_body(error);
            }
        };

        // Validate session exists (do NOT create if missing)
        if let Err(err) = self.validate_session_exists(&session_id).await {
            error!("Session validation failed for Session ID {}: {}", session_id, err);
            let error = JsonRpcError::new(
                None,
                JsonRpcErrorObject::server_error(
                    -32003,
                    &format!("Session validation failed: {}", err),
                    None
                )
            );
            return jsonrpc_error_to_unified_body(error);
        }

        // Extract Last-Event-ID for resumability
        let last_event_id = extract_last_event_id(headers);

        // Generate unique connection ID for MCP spec compliance
        let connection_id = Uuid::now_v7().to_string();

        debug!("Creating SSE stream for session: {} with connection: {}, last_event_id: {:?}",
               session_id, connection_id, last_event_id);

        // ✅ CORRECTED ARCHITECTURE: Use shared StreamManager directly (no registry needed)
        match self.stream_manager.handle_sse_connection(
            session_id,
            connection_id,
            last_event_id,
        ).await {
            Ok(response) => Ok(response),
            Err(err) => {
                error!("Failed to create SSE connection: {}", err);
                let error = JsonRpcError::new(
                    None,
                    JsonRpcErrorObject::internal_error(
                        Some(format!("SSE connection failed: {}", err))
                    )
                );
                jsonrpc_error_to_unified_body(error)
            }
        }
    }

    /// Handle DELETE requests for session cleanup
    async fn handle_delete_request(
        &self,
        req: Request<hyper::body::Incoming>,
    ) -> Result<Response<JsonRpcBody>> {
        let session_id = extract_session_id(req.headers());

        debug!("DELETE request - Session: {:?}", session_id);

        if let Some(session_id) = session_id {
            match self.session_storage.delete_session(&session_id).await {
                Ok(true) => {
                    debug!("Session {} removed via DELETE", session_id);
                    Ok(Response::builder()
                        .status(StatusCode::OK)
                        .body(Full::new(Bytes::from("Session removed")))
                        .unwrap())
                }
                Ok(false) => {
                    Ok(Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(Full::new(Bytes::from("Session not found")))
                        .unwrap())
                }
                Err(err) => {
                    error!("Error deleting session {}: {}", session_id, err);
                    Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Full::new(Bytes::from("Session deletion error")))
                        .unwrap())
                }
            }
        } else {
            Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Missing Mcp-Session-Id header")))
                .unwrap())
        }
    }

    /// Handle OPTIONS preflight requests - these are essential for CORS
    fn handle_preflight(&self) -> Response<JsonRpcBody> {
        options_response()
    }

    /// Return method not allowed response
    fn method_not_allowed(&self) -> Response<JsonRpcBody> {
        method_not_allowed_response()
    }

    /// Validate that a session exists - do NOT create if missing
    async fn validate_session_exists(&self, session_id: &str) -> Result<()> {
        // Check if session already exists
        match self.session_storage.get_session(session_id).await {
            Ok(Some(_)) => {
                debug!("Session validation successful: {}", session_id);
                Ok(())
            }
            Ok(None) => {
                error!("Session not found: {}", session_id);
                Err(crate::HttpMcpError::InvalidRequest(
                    format!("Session '{}' not found. Sessions must be created via initialize request first.", session_id)
                ))
            }
            Err(err) => {
                error!("Failed to validate session {}: {}", session_id, err);
                Err(crate::HttpMcpError::InvalidRequest(format!("Session validation failed: {}", err)))
            }
        }
    }
}
