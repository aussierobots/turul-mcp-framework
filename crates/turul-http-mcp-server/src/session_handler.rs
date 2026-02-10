//! JSON-RPC 2.0 over HTTP handler for MCP requests with SessionStorage integration
//!
//! This handler implements proper JSON-RPC 2.0 server over HTTP transport with
//! MCP 2025-11-25 compliance, including:
//! - SessionStorage trait integration (defaults to InMemory)
//! - StreamManager for SSE with resumability
//! - 202 Accepted for notifications
//! - Last-Event-ID header support
//! - Per-session event targeting

use std::convert::Infallible;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tracing::{debug, error, warn};

use bytes::Bytes;
use futures::Stream;
use http_body::{Body, Frame};
use http_body_util::{BodyExt, Full};
use hyper::header::{ACCEPT, CONTENT_TYPE};
use hyper::{Method, Request, Response, StatusCode};

use chrono;
use turul_mcp_json_rpc_server::{
    JsonRpcDispatcher,
    r#async::SessionContext,
    dispatch::{JsonRpcMessage, JsonRpcMessageResult, parse_json_rpc_message},
    error::{JsonRpcError, JsonRpcErrorObject},
};
use turul_mcp_protocol::McpError;
use turul_mcp_protocol::ServerCapabilities;
use turul_mcp_session_storage::{InMemorySessionStorage, SessionView};
use uuid::Uuid;

use crate::{
    Result, ServerConfig, StreamConfig, StreamManager,
    json_rpc_responses::*,
    notification_bridge::{SharedNotificationBroadcaster, StreamManagerNotificationBroadcaster},
    protocol::{extract_last_event_id, extract_protocol_version, extract_session_id, normalize_header_value},
};
use std::collections::HashMap;

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
        debug!("DROP: SessionSseStream - HTTP response body being cleaned up");
        debug!("This may indicate early cleanup of SSE response stream");
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
            Poll::Ready(Some(Ok(data))) => Poll::Ready(Some(Ok(Frame::data(data)))),
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

/// Accept header compliance mode for MCP Streamable HTTP
#[derive(Debug, Clone, PartialEq)]
enum AcceptMode {
    /// Client sends both application/json and text/event-stream (MCP spec compliant)
    Compliant,
    /// Client sends only application/json (compatibility mode for non-compliant clients)
    JsonOnly,
    /// Client sends only text/event-stream (SSE only)
    SseOnly,
    /// Client sends neither or something else entirely
    Invalid,
}

/// Parse MCP Accept header and determine compliance mode
fn parse_mcp_accept_header(accept_header: &str) -> (AcceptMode, bool) {
    let accepts_json = accept_header.contains("application/json") || accept_header.contains("*/*");
    let accepts_sse = accept_header.contains("text/event-stream");

    let mode = match (accepts_json, accepts_sse) {
        (true, true) => AcceptMode::Compliant,
        (true, false) => AcceptMode::JsonOnly, // MCP Inspector case
        (false, true) => AcceptMode::SseOnly,
        (false, false) => AcceptMode::Invalid,
    };

    // For SSE decision, we need both compliance and actual SSE support
    // In JsonOnly mode, we never use SSE even if server would prefer it
    let should_use_sse = match mode {
        AcceptMode::Compliant => true, // Server can choose
        AcceptMode::JsonOnly => false, // Force JSON for compatibility
        AcceptMode::SseOnly => true,   // Force SSE
        AcceptMode::Invalid => false,  // Fallback to JSON
    };

    (mode, should_use_sse)
}

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
pub struct SessionMcpHandler {
    pub(crate) config: ServerConfig,
    pub(crate) dispatcher: Arc<JsonRpcDispatcher<McpError>>,
    pub(crate) session_storage: Arc<turul_mcp_session_storage::BoxedSessionStorage>,
    pub(crate) stream_config: StreamConfig,
    // ✅ CORRECTED ARCHITECTURE: Single shared StreamManager instance with internal session management
    pub(crate) stream_manager: Arc<StreamManager>,
    pub(crate) middleware_stack: Arc<crate::middleware::MiddlewareStack>,
}

impl Clone for SessionMcpHandler {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            dispatcher: Arc::clone(&self.dispatcher),
            session_storage: Arc::clone(&self.session_storage),
            stream_config: self.stream_config.clone(),
            stream_manager: Arc::clone(&self.stream_manager),
            middleware_stack: Arc::clone(&self.middleware_stack),
        }
    }
}

impl SessionMcpHandler {
    /// Create a new handler with default in-memory storage (zero-configuration)
    pub fn new(
        config: ServerConfig,
        dispatcher: Arc<JsonRpcDispatcher<McpError>>,
        stream_config: StreamConfig,
    ) -> Self {
        let storage: Arc<turul_mcp_session_storage::BoxedSessionStorage> =
            Arc::new(InMemorySessionStorage::new());
        let middleware_stack = Arc::new(crate::middleware::MiddlewareStack::new());
        Self::with_storage(config, dispatcher, storage, stream_config, middleware_stack)
    }

    /// Create handler with shared StreamManager instance (corrected architecture)
    pub fn with_shared_stream_manager(
        config: ServerConfig,
        dispatcher: Arc<JsonRpcDispatcher<McpError>>,
        session_storage: Arc<turul_mcp_session_storage::BoxedSessionStorage>,
        stream_config: StreamConfig,
        stream_manager: Arc<StreamManager>,
        middleware_stack: Arc<crate::middleware::MiddlewareStack>,
    ) -> Self {
        Self {
            config,
            dispatcher,
            session_storage,
            stream_config,
            stream_manager,
            middleware_stack,
        }
    }

    /// Create handler with specific session storage backend (creates own StreamManager)
    /// Note: Use with_shared_stream_manager for correct architecture
    pub fn with_storage(
        config: ServerConfig,
        dispatcher: Arc<JsonRpcDispatcher<McpError>>,
        session_storage: Arc<turul_mcp_session_storage::BoxedSessionStorage>,
        stream_config: StreamConfig,
        middleware_stack: Arc<crate::middleware::MiddlewareStack>,
    ) -> Self {
        // Create own StreamManager instance (not recommended for production)
        let stream_manager = Arc::new(StreamManager::with_config(
            Arc::clone(&session_storage),
            stream_config.clone(),
        ));

        Self {
            config,
            dispatcher,
            session_storage,
            stream_config,
            stream_manager,
            middleware_stack,
        }
    }

    /// Get access to the StreamManager for notifications
    pub fn get_stream_manager(&self) -> &Arc<StreamManager> {
        &self.stream_manager
    }

    /// Handle MCP HTTP requests with full MCP 2025-11-25 compliance
    pub async fn handle_mcp_request<B>(&self, req: Request<B>) -> Result<Response<UnifiedMcpBody>>
    where
        B: http_body::Body<Data = bytes::Bytes, Error = hyper::Error> + Send + 'static,
    {
        debug!(
            "SESSION HANDLER processing {} {}",
            req.method(),
            req.uri().path()
        );
        match *req.method() {
            Method::POST => {
                let response = self.handle_json_rpc_request(req).await?;
                Ok(response)
            }
            Method::GET => self.handle_sse_request(req).await,
            Method::DELETE => {
                let response = self.handle_delete_request(req).await?;
                Ok(response.map(convert_to_unified_body))
            }
            Method::OPTIONS => {
                let response = self.handle_preflight();
                Ok(response.map(convert_to_unified_body))
            }
            _ => {
                let response = self.method_not_allowed();
                Ok(response.map(convert_to_unified_body))
            }
        }
    }

    /// Handle JSON-RPC requests over HTTP POST
    async fn handle_json_rpc_request<B>(&self, req: Request<B>) -> Result<Response<UnifiedMcpBody>>
    where
        B: http_body::Body<Data = bytes::Bytes, Error = hyper::Error> + Send + 'static,
    {
        // Extract all headers for middleware before body is consumed
        let headers: HashMap<String, String> = req
            .headers()
            .iter()
            .filter_map(|(k, v)| v.to_str().ok().map(|s| (k.as_str().to_string(), s.to_string())))
            .collect();

        // Extract protocol version and session ID from headers
        let protocol_version = extract_protocol_version(req.headers());
        let session_id = extract_session_id(req.headers());

        debug!(
            "POST request - Protocol: {}, Session: {:?}",
            protocol_version, session_id
        );

        // Check content type
        let content_type = req
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|ct| ct.to_str().ok())
            .map(normalize_header_value)
            .unwrap_or_default();

        if !content_type.starts_with("application/json") {
            warn!("Invalid content type: {}", content_type);
            return Ok(
                bad_request_response("Content-Type must be application/json")
                    .map(convert_to_unified_body),
            );
        }

        // Parse Accept header for MCP Streamable HTTP compliance
        let accept_header = req
            .headers()
            .get(ACCEPT)
            .and_then(|accept| accept.to_str().ok())
            .map(|v| normalize_header_value(v))
            .unwrap_or_else(|| "application/json".to_string());

        let (accept_mode, accepts_sse) = parse_mcp_accept_header(&accept_header);
        debug!(
            "POST request Accept header: '{}', mode: {:?}, will use SSE for tool calls: {}",
            accept_header, accept_mode, accepts_sse
        );

        // Read request body
        let body = req.into_body();
        let body_bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(err) => {
                error!("Failed to read request body: {}", err);
                return Ok(bad_request_response("Failed to read request body")
                    .map(convert_to_unified_body));
            }
        };

        // Check body size
        if body_bytes.len() > self.config.max_body_size {
            warn!("Request body too large: {} bytes", body_bytes.len());
            return Ok(Response::builder()
                .status(StatusCode::PAYLOAD_TOO_LARGE)
                .header(CONTENT_TYPE, "application/json")
                .body(convert_to_unified_body(Full::new(Bytes::from(
                    "Request body too large",
                ))))
                .unwrap());
        }

        // Parse as UTF-8
        let body_str = match std::str::from_utf8(&body_bytes) {
            Ok(s) => s,
            Err(err) => {
                error!("Invalid UTF-8 in request body: {}", err);
                return Ok(bad_request_response("Request body must be valid UTF-8")
                    .map(convert_to_unified_body));
            }
        };

        debug!("Received JSON-RPC request: {}", body_str);

        // Parse JSON-RPC message
        let message = match parse_json_rpc_message(body_str) {
            Ok(msg) => msg,
            Err(rpc_err) => {
                error!("JSON-RPC parse error: {}", rpc_err);
                // Extract request ID from the error if available
                let error_response =
                    serde_json::to_string(&rpc_err).unwrap_or_else(|_| "{}".to_string());
                return Ok(Response::builder()
                    .status(StatusCode::OK) // JSON-RPC parse errors still use 200 OK
                    .header(CONTENT_TYPE, "application/json")
                    .body(convert_to_unified_body(Full::new(Bytes::from(
                        error_response,
                    ))))
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
                    debug!(
                        "Handling initialize request - creating new session via session storage"
                    );

                    // Let session storage create the session and generate the ID (GPS pattern)
                    let capabilities = ServerCapabilities::default();
                    match self.session_storage.create_session(capabilities).await {
                        Ok(session_info) => {
                            debug!(
                                "Created new session via session storage: {}",
                                session_info.session_id
                            );

                            // ✅ CORRECTED ARCHITECTURE: Create session-specific notification broadcaster from shared StreamManager
                            let broadcaster: SharedNotificationBroadcaster =
                                Arc::new(StreamManagerNotificationBroadcaster::new(Arc::clone(
                                    &self.stream_manager,
                                )));
                            let broadcaster_any =
                                Arc::new(broadcaster) as Arc<dyn std::any::Any + Send + Sync>;

                            let session_context = SessionContext {
                                session_id: session_info.session_id.clone(),
                                metadata: std::collections::HashMap::new(),
                                broadcaster: Some(broadcaster_any),
                                timestamp: chrono::Utc::now().timestamp_millis() as u64,
                            };

                            // Run middleware pipeline and dispatch
                            // Injection is applied immediately inside run_middleware_and_dispatch
                            let (response, _) = self
                                .run_middleware_and_dispatch(request, headers.clone(), session_context)
                                .await;

                            // Return the session ID created by session storage for the HTTP header
                            (response, Some(session_info.session_id))
                        }
                        Err(err) => {
                            error!("Failed to create session during initialize: {}", err);
                            // Return error response using proper JSON-RPC error format
                            let error_msg = format!("Session creation failed: {}", err);
                            let error_response = turul_mcp_json_rpc_server::JsonRpcMessage::error(
                                turul_mcp_json_rpc_server::JsonRpcError::internal_error(
                                    Some(request.id),
                                    Some(error_msg),
                                ),
                            );
                            (error_response, None)
                        }
                    }
                } else {
                    // For non-initialize requests, create session context if session ID is provided
                    // Let server-level handlers decide whether to enforce session requirements
                    let session_context = if let Some(ref session_id_str) = session_id {
                        debug!("Processing request with session: {}", session_id_str);
                        let broadcaster: SharedNotificationBroadcaster =
                            Arc::new(StreamManagerNotificationBroadcaster::new(Arc::clone(
                                &self.stream_manager,
                            )));
                        let broadcaster_any =
                            Arc::new(broadcaster) as Arc<dyn std::any::Any + Send + Sync>;
                        Some(SessionContext {
                            session_id: session_id_str.clone(),
                            metadata: std::collections::HashMap::new(),
                            broadcaster: Some(broadcaster_any),
                            timestamp: chrono::Utc::now().timestamp_millis() as u64,
                        })
                    } else {
                        debug!("Processing request without session (lenient mode)");
                        None
                    };

                    // Run middleware pipeline and dispatch
                    let (response, _stashed_injection) = if let Some(ctx) = session_context {
                        self.run_middleware_and_dispatch(request, headers.clone(), ctx).await
                    } else {
                        // No session - fast path (no middleware, just dispatch)
                        (self.dispatcher.handle_request(request).await, None)
                    };
                    (response, session_id)
                };

                // Convert JsonRpcMessage to JsonRpcMessageResult
                let message_result = match response {
                    turul_mcp_json_rpc_server::JsonRpcMessage::Response(resp) => {
                        JsonRpcMessageResult::Response(resp)
                    }
                    turul_mcp_json_rpc_server::JsonRpcMessage::Error(err) => {
                        JsonRpcMessageResult::Error(err)
                    }
                };
                (message_result, response_session_id, Some(method_name))
            }
            JsonRpcMessage::Notification(notification) => {
                debug!(
                    "Processing JSON-RPC notification: method={}",
                    notification.method
                );
                let method_name = notification.method.clone();

                // For notifications, create session context if session ID is provided
                // Let server-level handlers decide whether to enforce session requirements
                let session_context = if let Some(ref session_id_str) = session_id {
                    debug!("Processing notification with session: {}", session_id_str);
                    let broadcaster: SharedNotificationBroadcaster = Arc::new(
                        StreamManagerNotificationBroadcaster::new(Arc::clone(&self.stream_manager)),
                    );
                    let broadcaster_any =
                        Arc::new(broadcaster) as Arc<dyn std::any::Any + Send + Sync>;

                    Some(SessionContext {
                        session_id: session_id_str.clone(),
                        metadata: std::collections::HashMap::new(),
                        broadcaster: Some(broadcaster_any),
                        timestamp: chrono::Utc::now().timestamp_millis() as u64,
                    })
                } else {
                    debug!("Processing notification without session (lenient mode)");
                    None
                };

                let result = self
                    .dispatcher
                    .handle_notification_with_context(notification, session_context)
                    .await;

                if let Err(err) = result {
                    error!("Notification handling error: {}", err);
                }
                (
                    JsonRpcMessageResult::NoResponse,
                    session_id.clone(),
                    Some(method_name),
                )
            }
        };

        // Convert message result to HTTP response
        match message_result {
            JsonRpcMessageResult::Response(response) => {
                // Check if this is a tool call that should return SSE
                // Only use SSE if explicitly requested via Accept: text/event-stream header
                let is_tool_call = method_name.as_ref().is_some_and(|m| m == "tools/call");

                debug!(
                    "Decision point: method={:?}, accept_mode={:?}, accepts_sse={}, server_post_sse_enabled={}, session_id={:?}, is_tool_call={}",
                    method_name,
                    accept_mode,
                    accepts_sse,
                    self.config.enable_post_sse,
                    response_session_id,
                    is_tool_call
                );

                // MCP Streamable HTTP decision logic based on Accept header compliance AND server configuration
                let should_use_sse = match accept_mode {
                    AcceptMode::JsonOnly => false, // Force JSON for compatibility (MCP Inspector)
                    AcceptMode::Invalid => false,  // Fallback to JSON for invalid headers
                    AcceptMode::Compliant => {
                        self.config.enable_post_sse && accepts_sse && is_tool_call
                    } // Server chooses for compliant clients
                    AcceptMode::SseOnly => self.config.enable_post_sse && accepts_sse, // Force SSE if server allows and client accepts
                };

                if should_use_sse && response_session_id.is_some() {
                    debug!(
                        "📡 Creating POST SSE stream (mode: {:?}) for tool call with notifications",
                        accept_mode
                    );
                    match self
                        .stream_manager
                        .create_post_sse_stream(
                            response_session_id.clone().unwrap(),
                            response.clone(), // Clone the response for SSE stream creation
                        )
                        .await
                    {
                        Ok(sse_response) => {
                            debug!("✅ POST SSE stream created successfully");
                            Ok(sse_response
                                .map(|body| body.map_err(|never| match never {}).boxed_unsync()))
                        }
                        Err(e) => {
                            warn!(
                                "Failed to create POST SSE stream, falling back to JSON: {}",
                                e
                            );
                            Ok(
                                jsonrpc_response_with_session(response, response_session_id)?
                                    .map(convert_to_unified_body),
                            )
                        }
                    }
                } else {
                    debug!(
                        "📄 Returning standard JSON response (mode: {:?}) for method: {:?}",
                        accept_mode, method_name
                    );
                    Ok(
                        jsonrpc_response_with_session(response, response_session_id)?
                            .map(convert_to_unified_body),
                    )
                }
            }
            JsonRpcMessageResult::Error(error) => {
                warn!("Sending JSON-RPC error response");
                // Convert JsonRpcError to proper HTTP response
                let error_json = serde_json::to_string(&error)?;
                Ok(Response::builder()
                    .status(StatusCode::OK) // JSON-RPC errors still return 200 OK
                    .header(CONTENT_TYPE, "application/json")
                    .body(convert_to_unified_body(Full::new(Bytes::from(error_json))))
                    .unwrap())
            }
            JsonRpcMessageResult::NoResponse => {
                // Notifications don't return responses (204 No Content)
                Ok(jsonrpc_notification_response()?.map(convert_to_unified_body))
            }
        }
    }

    // Note: create_post_sse_response method removed as it's unused in MCP Inspector compatibility mode
    // SSE for tool calls is temporarily disabled - see WORKING_MEMORY.md for details

    /// Handle Server-Sent Events requests (SSE for streaming)
    async fn handle_sse_request<B>(&self, req: Request<B>) -> Result<Response<UnifiedMcpBody>>
    where
        B: http_body::Body<Data = bytes::Bytes, Error = hyper::Error> + Send + 'static,
    {
        // Check if client accepts SSE
        let headers = req.headers();
        let accept = headers
            .get(ACCEPT)
            .and_then(|accept| accept.to_str().ok())
            .map(normalize_header_value)
            .unwrap_or_default();

        if !accept.contains("text/event-stream") {
            warn!(
                "GET request received without SSE support - header does not contain 'text/event-stream'"
            );
            let error = JsonRpcError::new(
                None,
                JsonRpcErrorObject::server_error(
                    -32001,
                    "SSE not accepted - missing 'text/event-stream' in Accept header",
                    None,
                ),
            );
            return jsonrpc_error_to_unified_body(error);
        }

        // Check if GET SSE is enabled on the server
        if !self.config.enable_get_sse {
            warn!("GET SSE request received but GET SSE is disabled on server");
            let error = JsonRpcError::new(
                None,
                JsonRpcErrorObject::server_error(
                    -32003,
                    "GET SSE is disabled on this server",
                    None,
                ),
            );
            return jsonrpc_error_to_unified_body(error);
        }

        // Extract protocol version and session ID
        let protocol_version = extract_protocol_version(headers);
        let session_id = extract_session_id(headers);

        debug!(
            "GET SSE request - Protocol: {}, Session: {:?}",
            protocol_version, session_id
        );

        // Session ID is required for SSE
        let session_id = match session_id {
            Some(id) => id,
            None => {
                warn!("Missing Mcp-Session-Id header for SSE request");
                let error = JsonRpcError::new(
                    None,
                    JsonRpcErrorObject::server_error(-32002, "Missing Mcp-Session-Id header", None),
                );
                return jsonrpc_error_to_unified_body(error);
            }
        };

        // Validate session exists (do NOT create if missing)
        if let Err(err) = self.validate_session_exists(&session_id).await {
            error!(
                "Session validation failed for Session ID {}: {}",
                session_id, err
            );
            let error = JsonRpcError::new(
                None,
                JsonRpcErrorObject::server_error(
                    -32003,
                    &format!("Session validation failed: {}", err),
                    None,
                ),
            );
            return jsonrpc_error_to_unified_body(error);
        }

        // Extract Last-Event-ID for resumability
        let last_event_id = extract_last_event_id(headers);

        // Generate unique connection ID for MCP spec compliance
        let connection_id = Uuid::now_v7().to_string();

        debug!(
            "Creating SSE stream for session: {} with connection: {}, last_event_id: {:?}",
            session_id, connection_id, last_event_id
        );

        // ✅ CORRECTED ARCHITECTURE: Use shared StreamManager directly (no registry needed)
        match self
            .stream_manager
            .handle_sse_connection(session_id, connection_id, last_event_id)
            .await
        {
            Ok(response) => Ok(response),
            Err(err) => {
                error!("Failed to create SSE connection: {}", err);
                let error = JsonRpcError::new(
                    None,
                    JsonRpcErrorObject::internal_error(Some(format!(
                        "SSE connection failed: {}",
                        err
                    ))),
                );
                jsonrpc_error_to_unified_body(error)
            }
        }
    }

    /// Handle DELETE requests for session cleanup
    async fn handle_delete_request<B>(&self, req: Request<B>) -> Result<Response<JsonRpcBody>>
    where
        B: http_body::Body<Data = bytes::Bytes, Error = hyper::Error> + Send + 'static,
    {
        let session_id = extract_session_id(req.headers());

        debug!("DELETE request - Session: {:?}", session_id);

        if let Some(session_id) = session_id {
            // First, close any active SSE connections for this session
            let closed_connections = self
                .stream_manager
                .close_session_connections(&session_id)
                .await;
            debug!(
                "Closed {} SSE connections for session: {}",
                closed_connections, session_id
            );

            // Mark session as terminated instead of immediate deletion (for proper lifecycle management)
            match self.session_storage.get_session(&session_id).await {
                Ok(Some(mut session_info)) => {
                    // Mark session as terminated in state
                    session_info
                        .state
                        .insert("terminated".to_string(), serde_json::Value::Bool(true));
                    session_info.state.insert(
                        "terminated_at".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(
                            chrono::Utc::now().timestamp_millis(),
                        )),
                    );
                    session_info.touch();

                    match self.session_storage.update_session(session_info).await {
                        Ok(()) => {
                            debug!(
                                "Session {} marked as terminated (TTL will handle cleanup)",
                                session_id
                            );
                            Ok(Response::builder()
                                .status(StatusCode::OK)
                                .body(Full::new(Bytes::from("Session terminated")))
                                .unwrap())
                        }
                        Err(err) => {
                            error!(
                                "Error marking session {} as terminated: {}",
                                session_id, err
                            );
                            // Fallback to deletion if update fails
                            match self.session_storage.delete_session(&session_id).await {
                                Ok(_) => {
                                    debug!("Session {} deleted as fallback", session_id);
                                    Ok(Response::builder()
                                        .status(StatusCode::OK)
                                        .body(Full::new(Bytes::from("Session removed")))
                                        .unwrap())
                                }
                                Err(delete_err) => {
                                    error!(
                                        "Error deleting session {} as fallback: {}",
                                        session_id, delete_err
                                    );
                                    Ok(Response::builder()
                                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                                        .body(Full::new(Bytes::from("Session termination error")))
                                        .unwrap())
                                }
                            }
                        }
                    }
                }
                Ok(None) => Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Full::new(Bytes::from("Session not found")))
                    .unwrap()),
                Err(err) => {
                    error!(
                        "Error retrieving session {} for termination: {}",
                        session_id, err
                    );
                    Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Full::new(Bytes::from("Session lookup error")))
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
                Err(crate::HttpMcpError::InvalidRequest(format!(
                    "Session '{}' not found. Sessions must be created via initialize request first.",
                    session_id
                )))
            }
            Err(err) => {
                error!("Failed to validate session {}: {}", session_id, err);
                Err(crate::HttpMcpError::InvalidRequest(format!(
                    "Session validation failed: {}",
                    err
                )))
            }
        }
    }

    /// Helper method to run middleware pipeline and dispatch request
    /// Shared logic between StreamableHttpHandler and SessionMcpHandler
    async fn run_middleware_and_dispatch(
        &self,
        request: turul_mcp_json_rpc_server::JsonRpcRequest,
        headers: HashMap<String, String>,
        session: turul_mcp_json_rpc_server::SessionContext,
    ) -> (turul_mcp_json_rpc_server::JsonRpcMessage, Option<crate::middleware::SessionInjection>) {
        // Fast path: if middleware stack is empty, dispatch directly
        if self.middleware_stack.is_empty() {
            let result = self.dispatcher
                .handle_request_with_context(request, session)
                .await;
            return (result, None);
        }

        // Normalize headers: lowercase String keys
        let normalized_headers: HashMap<String, String> = headers
            .iter()
            .map(|(k, v)| (k.to_lowercase(), v.clone()))
            .collect();

        // Build RequestContext with method and headers
        // Clone method and session_id for ctx (request will be moved to dispatcher)
        let method = request.method.clone();
        let session_id = session.session_id.clone();

        // Convert params to Option<Value>
        let params = request.params.clone().map(|p| match p {
            turul_mcp_json_rpc_server::RequestParams::Object(map) => {
                serde_json::Value::Object(map.into_iter().collect())
            }
            turul_mcp_json_rpc_server::RequestParams::Array(arr) => serde_json::Value::Array(arr),
        });
        let mut ctx = crate::middleware::RequestContext::new(&method, params);

        for (k, v) in normalized_headers {
            ctx.add_metadata(k, serde_json::json!(v));
        }

        // Create SessionView adapter for middleware to access storage-backed session
        let session_view = crate::middleware::StorageBackedSessionView::new(
            session_id.clone(),
            Arc::clone(&self.session_storage),
        );

        // Execute before_dispatch with SessionView
        let injection = match self.middleware_stack.execute_before(&mut ctx, Some(&session_view)).await {
            Ok(inj) => inj,
            Err(err) => {
                // Map middleware error to proper JSON-RPC error code
                return (Self::map_middleware_error_to_jsonrpc(err, request.id), None);
            }
        };

        // Apply injection immediately to session storage
        if !injection.is_empty() {
            for (key, value) in injection.state() {
                if let Err(e) = session_view.set_state(key, value.clone()).await {
                    tracing::warn!("Failed to apply injection state '{}': {}", key, e);
                }
            }
            for (key, value) in injection.metadata() {
                if let Err(e) = session_view.set_metadata(key, value.clone()).await {
                    tracing::warn!("Failed to apply injection metadata '{}': {}", key, e);
                }
            }
        }

        // Dispatch the request
        let result = self.dispatcher
            .handle_request_with_context(request, session)
            .await;

        // Execute after_dispatch
        // Convert JsonRpcMessage to DispatcherResult for middleware
        let mut dispatcher_result = match &result {
            turul_mcp_json_rpc_server::JsonRpcMessage::Response(resp) => {
                match &resp.result {
                    turul_mcp_json_rpc_server::response::ResponseResult::Success(val) => {
                        crate::middleware::DispatcherResult::Success(val.clone())
                    }
                    turul_mcp_json_rpc_server::response::ResponseResult::Null => {
                        crate::middleware::DispatcherResult::Success(serde_json::Value::Null)
                    }
                }
            }
            turul_mcp_json_rpc_server::JsonRpcMessage::Error(err) => {
                crate::middleware::DispatcherResult::Error(err.error.message.clone())
            }
        };

        // Ignore errors from after_dispatch (they shouldn't prevent returning the result)
        let _ = self.middleware_stack.execute_after(&ctx, &mut dispatcher_result).await;

        (result, None) // Injection already applied, no need to return it
    }

    /// Map MiddlewareError to JSON-RPC error with semantic error codes
    fn map_middleware_error_to_jsonrpc(
        err: crate::middleware::MiddlewareError,
        request_id: turul_mcp_json_rpc_server::RequestId,
    ) -> turul_mcp_json_rpc_server::JsonRpcMessage {
        use crate::middleware::error::error_codes;
        use crate::middleware::MiddlewareError;

        let (code, message, data) = match err {
            MiddlewareError::Unauthenticated(msg) => (error_codes::UNAUTHENTICATED, msg, None),
            MiddlewareError::Unauthorized(msg) => (error_codes::UNAUTHORIZED, msg, None),
            MiddlewareError::RateLimitExceeded {
                message,
                retry_after,
            } => {
                let data = retry_after.map(|s| serde_json::json!({"retryAfter": s}));
                (error_codes::RATE_LIMIT_EXCEEDED, message, data)
            }
            MiddlewareError::InvalidRequest(msg) => (error_codes::INVALID_REQUEST, msg, None),
            MiddlewareError::Internal(msg) => (error_codes::INTERNAL_ERROR, msg, None),
            MiddlewareError::Custom { message, .. } => (error_codes::INTERNAL_ERROR, message, None),
        };

        let error_obj = if let Some(d) = data {
            turul_mcp_json_rpc_server::error::JsonRpcErrorObject::server_error(code, &message, Some(d))
        } else {
            turul_mcp_json_rpc_server::error::JsonRpcErrorObject::server_error(
                code,
                &message,
                None::<serde_json::Value>,
            )
        };

        turul_mcp_json_rpc_server::JsonRpcMessage::Error(turul_mcp_json_rpc_server::JsonRpcError::new(
            Some(request_id),
            error_obj,
        ))
    }
}
