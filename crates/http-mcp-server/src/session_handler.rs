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

use hyper::{Request, Response, Method, StatusCode};
use bytes::Bytes;
use hyper::header::{CONTENT_TYPE, ACCEPT};
use http_body_util::{BodyExt, Full};
use http_body::{Body, Frame};
use tracing::{debug, warn, error};
use futures::Stream;

use mcp_json_rpc_server::{
    JsonRpcDispatcher,
    r#async::SessionContext,
    dispatch::{parse_json_rpc_message, JsonRpcMessage, JsonRpcMessageResult},
    error::{JsonRpcError, JsonRpcErrorObject},
    JsonRpcResponse
};
use mcp_session_storage::{SessionStorage, InMemorySessionStorage};
use mcp_protocol::ServerCapabilities;
use chrono;

use crate::{
    Result, ServerConfig, StreamConfig,
    protocol::{extract_protocol_version, extract_session_id, extract_last_event_id}, 
    json_rpc_responses::*,
    StreamManager,
    notification_bridge::{SharedNotificationBroadcaster, StreamManagerNotificationBroadcaster}
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

/// JSON-RPC 2.0 over HTTP handler with SessionStorage integration and SSE streaming bridge
pub struct SessionMcpHandler<S: SessionStorage = InMemorySessionStorage> {
    pub(crate) config: ServerConfig,
    pub(crate) dispatcher: Arc<JsonRpcDispatcher>,
    pub(crate) session_storage: Arc<S>,
    pub(crate) stream_config: StreamConfig,
    pub(crate) stream_manager: Arc<StreamManager<S>>,
    pub(crate) notification_broadcaster: SharedNotificationBroadcaster,
}

impl<S: SessionStorage + 'static> Clone for SessionMcpHandler<S> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            dispatcher: Arc::clone(&self.dispatcher),
            session_storage: Arc::clone(&self.session_storage),
            stream_config: self.stream_config.clone(),
            stream_manager: Arc::clone(&self.stream_manager),
            notification_broadcaster: Arc::clone(&self.notification_broadcaster),
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
        let stream_manager = Arc::new(StreamManager::with_config(Arc::clone(&storage), stream_config.clone()));
        
        // 🔑 THE CRITICAL BRIDGE: Connect NotificationBroadcaster to StreamManager
        let notification_broadcaster: SharedNotificationBroadcaster = Arc::new(
            StreamManagerNotificationBroadcaster::new(Arc::clone(&stream_manager))
        );
        
        Self { 
            config, 
            dispatcher, 
            session_storage: storage, 
            stream_config, 
            stream_manager, 
            notification_broadcaster 
        }
    }
}

impl<S: SessionStorage + 'static> SessionMcpHandler<S> {
    /// Create handler with specific session storage backend
    pub fn with_storage(
        config: ServerConfig, 
        dispatcher: Arc<JsonRpcDispatcher>,
        session_storage: Arc<S>,
        stream_config: StreamConfig,
    ) -> Self {
        let stream_manager = Arc::new(StreamManager::with_config(Arc::clone(&session_storage), stream_config.clone()));
        
        // 🔑 THE CRITICAL BRIDGE: Connect NotificationBroadcaster to StreamManager
        let notification_broadcaster: SharedNotificationBroadcaster = Arc::new(
            StreamManagerNotificationBroadcaster::new(Arc::clone(&stream_manager))
        );
        
        Self { 
            config, 
            dispatcher, 
            session_storage, 
            stream_config, 
            stream_manager, 
            notification_broadcaster 
        }
    }

    /// Get access to the StreamManager for external event forwarding
    /// This allows the mcp-server layer to forward session events to SSE streams
    pub fn get_stream_manager(&self) -> &Arc<StreamManager<S>> {
        &self.stream_manager
    }
    
    /// Get access to the NotificationBroadcaster for tools to send notifications
    /// This is the bridge that connects tool notifications to SSE streams
    pub fn get_notification_broadcaster(&self) -> &SharedNotificationBroadcaster {
        &self.notification_broadcaster
    }
    
    /// Helper to get broadcaster as Any for SessionContext
    fn get_broadcaster_as_any(&self) -> Arc<dyn std::any::Any + Send + Sync> {
        // Create a new Arc that implements Any
        Arc::new(Arc::clone(&self.notification_broadcaster))
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
            .unwrap_or("");
        
        let accepts_sse = accept_header.contains("text/event-stream");

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
        let (message_result, response_session_id) = match message {
            JsonRpcMessage::Request(request) => {
                debug!("Processing JSON-RPC request: method={}", request.method);
                
                // Special handling for initialize requests - they create new sessions
                let (response, response_session_id) = if request.method == "initialize" {
                    debug!("Handling initialize request - creating new session via session storage");
                    
                    // Let session storage create the session and generate the ID (GPS pattern)
                    let capabilities = ServerCapabilities::default();
                    match self.session_storage.create_session(capabilities).await {
                        Ok(session_info) => {
                            debug!("Created new session via session storage: {}", session_info.session_id);
                            
                            let session_context = SessionContext {
                                session_id: session_info.session_id.clone(),
                                metadata: std::collections::HashMap::new(),
                                broadcaster: Some(self.get_broadcaster_as_any()),
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
                            let error_response = mcp_json_rpc_server::JsonRpcResponse::success(
                                request.id,
                                serde_json::json!({"error": error_msg})
                            );
                            (error_response, None)
                        }
                    }
                } else {
                    // Regular requests use existing session context
                    let session_context = SessionContext {
                        session_id: session_id.clone().unwrap_or("unknown".to_string()),
                        metadata: std::collections::HashMap::new(),
                        broadcaster: Some(self.get_broadcaster_as_any()),
                        timestamp: chrono::Utc::now().timestamp_millis() as u64,
                    };
                    
                    let response = self.dispatcher.handle_request_with_context(request, session_context).await;
                    (response, session_id)
                };
                
                (JsonRpcMessageResult::Response(response), response_session_id)
            }
            JsonRpcMessage::Notification(notification) => {
                debug!("Processing JSON-RPC notification: method={}", notification.method);
                if let Err(err) = self.dispatcher.handle_notification(notification).await {
                    error!("Notification handling error: {}", err);
                }
                (JsonRpcMessageResult::NoResponse, None)
            }
        };

        // Convert message result to HTTP response
        match message_result {
            JsonRpcMessageResult::Response(response) => {
                if accepts_sse && response_session_id.is_some() {
                    // MCP Streamable HTTP: Return SSE stream with the response
                    debug!("Client accepts SSE - creating streaming response");
                    self.create_post_sse_response(response, response_session_id.unwrap()).await
                } else {
                    // Standard JSON response
                    debug!("Sending JSON-RPC response");
                    Ok(jsonrpc_response_with_session(response, response_session_id)?)
                }
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

    /// Create SSE response for POST request (MCP Streamable HTTP)
    async fn create_post_sse_response(
        &self,
        response: JsonRpcResponse,
        session_id: String,
    ) -> Result<Response<JsonRpcBody>> {
        debug!("Creating POST SSE stream for session: {}", session_id);
        
        // Create the SSE stream using StreamManager
        // For POST requests, we return a stream that:
        // 1. Sends any notifications related to the request
        // 2. Sends the JSON-RPC response
        // 3. Closes the stream after response
        match self.stream_manager.create_post_sse_stream(
            session_id.clone(),
            response.clone(),
        ).await {
            Ok(sse_response) => {
                debug!("Created POST SSE stream for session: {}", session_id);
                Ok(sse_response)
            }
            Err(err) => {
                error!("Failed to create POST SSE stream: {}", err);
                // Fall back to JSON response
                Ok(jsonrpc_response_with_session(response, Some(session_id))?)
            }
        }
    }

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
        
        debug!("Creating SSE stream for session: {} with last_event_id: {:?}", session_id, last_event_id);
        
        // Use StreamManager to create proper SSE connection (one per session per SSE spec)
        match self.stream_manager.handle_sse_connection(
            session_id,
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