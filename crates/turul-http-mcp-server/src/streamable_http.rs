//! Streamable HTTP Transport for MCP 2025-06-18
//!
//! This module implements the "Streamable HTTP" transport mechanism introduced
//! in MCP 2025-03-26, which replaces the previous HTTP+SSE approach from 2024-11-05.
//!
//! ## Key Improvements over HTTP+SSE
//! - **Serverless Compatibility**: Enables deployment on AWS Lambda, Google Cloud Run
//! - **Improved Scalability**: Supports chunked transfer encoding and progressive delivery
//! - **Session Management**: Cryptographically secure session IDs for connection tracking
//! - **Enterprise Network Friendly**: No long-lived connections or polling requirements

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;
use futures::Stream;
use http_body::Body;
use http_body_util::{BodyExt, Full};
use hyper::header::{ACCEPT, CONTENT_TYPE};
use hyper::{HeaderMap, Method, Request, Response, StatusCode};
use serde_json::Value;
use tracing::{debug, error, info, warn};

use crate::ServerConfig;

/// MCP Protocol versions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum McpProtocolVersion {
    /// Original protocol without streamable HTTP (2024-11-05)
    V2024_11_05,
    /// Protocol including streamable HTTP (2025-03-26)
    V2025_03_26,
    /// Protocol with structured _meta, cursor, progressToken, and elicitation (2025-06-18)
    #[default]
    V2025_06_18,
}

impl McpProtocolVersion {
    /// Parse from header string
    pub fn parse_version(s: &str) -> Option<Self> {
        match s {
            "2024-11-05" => Some(Self::V2024_11_05),
            "2025-03-26" => Some(Self::V2025_03_26),
            "2025-06-18" => Some(Self::V2025_06_18),
            _ => None,
        }
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::V2024_11_05 => "2024-11-05",
            Self::V2025_03_26 => "2025-03-26",
            Self::V2025_06_18 => "2025-06-18",
        }
    }

    /// Returns whether this version supports streamable HTTP
    pub fn supports_streamable_http(&self) -> bool {
        matches!(self, Self::V2025_03_26 | Self::V2025_06_18)
    }

    /// Returns whether this version supports _meta fields
    pub fn supports_meta_fields(&self) -> bool {
        matches!(self, Self::V2025_06_18)
    }

    /// Returns whether this version supports cursor-based pagination
    pub fn supports_cursors(&self) -> bool {
        matches!(self, Self::V2025_06_18)
    }

    /// Returns whether this version supports progress tokens
    pub fn supports_progress_tokens(&self) -> bool {
        matches!(self, Self::V2025_06_18)
    }

    /// Returns whether this version supports elicitation
    pub fn supports_elicitation(&self) -> bool {
        matches!(self, Self::V2025_06_18)
    }

    /// Get list of supported features for this version
    pub fn supported_features(&self) -> Vec<&'static str> {
        let mut features = vec![];
        if self.supports_streamable_http() {
            features.push("streamable-http");
        }
        if self.supports_meta_fields() {
            features.push("_meta-fields");
        }
        if self.supports_cursors() {
            features.push("cursor-pagination");
        }
        if self.supports_progress_tokens() {
            features.push("progress-tokens");
        }
        if self.supports_elicitation() {
            features.push("elicitation");
        }
        features
    }
}

/// Streamable HTTP request context
#[derive(Debug, Clone)]
pub struct StreamableHttpContext {
    /// Protocol version negotiated
    pub protocol_version: McpProtocolVersion,
    /// Session ID if provided
    pub session_id: Option<String>,
    /// Whether client wants SSE stream (text/event-stream)
    pub wants_sse_stream: bool,
    /// Whether client accepts stream frames (application/json or */*)
    pub accepts_stream_frames: bool,
    /// Additional request headers
    pub headers: HashMap<String, String>,
}

impl StreamableHttpContext {
    /// Parse context from HTTP request headers
    pub fn from_request<T>(req: &Request<T>) -> Self {
        let headers = req.headers();

        // Parse protocol version from MCP-Protocol-Version header
        let protocol_version = headers
            .get("MCP-Protocol-Version")
            .and_then(|h| h.to_str().ok())
            .and_then(McpProtocolVersion::parse_version)
            .unwrap_or_default();

        // Extract session ID from Mcp-Session-Id header (note capitalization)
        let session_id = headers
            .get("Mcp-Session-Id")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        // Check Accept header for streaming and JSON support
        let accept_header = headers
            .get(ACCEPT)
            .and_then(|h| h.to_str().ok())
            .unwrap_or_default()
            .to_ascii_lowercase();

        let wants_sse_stream = accept_header.contains("text/event-stream");
        let accepts_stream_frames =
            accept_header.contains("application/json") || accept_header.contains("*/*");

        // Collect additional headers for debugging/logging
        let mut header_map = HashMap::new();
        for (name, value) in headers.iter() {
            if let Ok(value_str) = value.to_str() {
                header_map.insert(name.to_string(), value_str.to_string());
            }
        }

        Self {
            protocol_version,
            session_id,
            wants_sse_stream,
            accepts_stream_frames,
            headers: header_map,
        }
    }

    /// Whether client wants SSE stream
    pub fn wants_sse_stream(&self) -> bool {
        self.wants_sse_stream
    }

    /// Whether client wants streaming POST responses
    pub fn wants_streaming_post(&self) -> bool {
        self.accepts_stream_frames && self.wants_sse_stream
    }

    /// Check if request is compatible with streamable HTTP
    pub fn is_streamable_compatible(&self) -> bool {
        self.protocol_version.supports_streamable_http()
            && self.accepts_stream_frames
    }

    /// Validate request for MCP compliance
    pub fn validate(&self) -> std::result::Result<(), String> {
        if !self.accepts_stream_frames {
            return Err("Accept header must include application/json".to_string());
        }

        if self.wants_sse_stream && !self.protocol_version.supports_streamable_http() {
            return Err(format!(
                "Protocol version {} does not support streamable HTTP",
                self.protocol_version.as_str()
            ));
        }

        // Only enforce session_id when SSE stream was explicitly requested
        if self.wants_sse_stream && self.session_id.is_none() {
            return Err("Mcp-Session-Id header required for streaming requests".to_string());
        }

        Ok(())
    }

    /// Create response headers for this context
    pub fn response_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();

        // Always include protocol version in response
        headers.insert(
            "MCP-Protocol-Version",
            self.protocol_version.as_str().parse().unwrap(),
        );

        // Include session ID if present
        if let Some(session_id) = &self.session_id {
            headers.insert("Mcp-Session-Id", session_id.parse().unwrap());
        }

        // Add capabilities header showing supported features
        let features = self.protocol_version.supported_features();
        if !features.is_empty() {
            headers.insert("MCP-Capabilities", features.join(",").parse().unwrap());
        }

        headers
    }
}

/// Streamable HTTP response types
pub enum StreamableResponse {
    /// Single JSON response
    Json(Value),
    /// Streaming response with multiple JSON messages
    Stream(Pin<Box<dyn Stream<Item = std::result::Result<Value, String>> + Send>>),
    /// Error response
    Error { status: StatusCode, message: String },
}

impl std::fmt::Debug for StreamableResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(value) => f.debug_tuple("Json").field(value).finish(),
            Self::Stream(_) => f.debug_tuple("Stream").field(&"<stream>").finish(),
            Self::Error { status, message } => f
                .debug_struct("Error")
                .field("status", status)
                .field("message", message)
                .finish(),
        }
    }
}

impl StreamableResponse {
    /// Convert to HTTP response
    pub fn into_response(self, context: &StreamableHttpContext) -> Response<Full<Bytes>> {
        let mut response_headers = context.response_headers();

        match self {
            StreamableResponse::Json(json) => {
                response_headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());

                let body = serde_json::to_string(&json)
                    .unwrap_or_else(|_| r#"{"error": "Failed to serialize response"}"#.to_string());

                Response::builder()
                    .status(StatusCode::OK)
                    .body(Full::new(Bytes::from(body)))
                    .unwrap()
            }

            StreamableResponse::Stream(_stream) => {
                // For streaming responses, set appropriate headers
                response_headers.insert(CONTENT_TYPE, "text/event-stream".parse().unwrap());
                response_headers.insert("Cache-Control", "no-cache, no-transform".parse().unwrap());
                response_headers.insert("Connection", "keep-alive".parse().unwrap());

                // TODO: Implement actual streaming body with chunked transfer encoding
                // Should stream JSON messages over HTTP with proper Content-Type: text/event-stream
                // For now, return 202 Accepted to indicate streaming would happen
                Response::builder()
                    .status(StatusCode::ACCEPTED)
                    .body(Full::new(Bytes::from("Streaming response accepted")))
                    .unwrap()
            }

            StreamableResponse::Error { status, message } => {
                response_headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());

                let error_json = serde_json::json!({
                    "error": {
                        "code": status.as_u16(),
                        "message": message
                    }
                });

                let body = serde_json::to_string(&error_json).unwrap_or_else(|_| {
                    r#"{"error": {"code": 500, "message": "Internal server error"}}"#.to_string()
                });

                Response::builder()
                    .status(status)
                    .body(Full::new(Bytes::from(body)))
                    .unwrap()
            }
        }
    }

    /// Convert to HTTP response with UnsyncBoxBody for streaming compatibility
    pub fn into_boxed_response(
        self,
        context: &StreamableHttpContext
    ) -> Response<http_body_util::combinators::UnsyncBoxBody<Bytes, hyper::Error>> {
        self.into_response(context)
            .map(|body| body.map_err(|never| match never {}).boxed_unsync())
    }
}

/// Streamable HTTP transport handler
#[derive(Clone)]
pub struct StreamableHttpHandler {
    config: Arc<ServerConfig>,
    dispatcher: Arc<turul_mcp_json_rpc_server::JsonRpcDispatcher<turul_mcp_protocol::McpError>>,
    session_storage: Arc<turul_mcp_session_storage::BoxedSessionStorage>,
    stream_manager: Arc<crate::StreamManager>,
}

impl StreamableHttpHandler {
    pub fn new(
        config: Arc<ServerConfig>,
        dispatcher: Arc<turul_mcp_json_rpc_server::JsonRpcDispatcher<turul_mcp_protocol::McpError>>,
        session_storage: Arc<turul_mcp_session_storage::BoxedSessionStorage>,
        stream_manager: Arc<crate::StreamManager>,
    ) -> Self {
        Self {
            config,
            dispatcher,
            session_storage,
            stream_manager,
        }
    }

    /// Handle incoming HTTP request with streamable HTTP support
    pub async fn handle_request<T>(&self, req: Request<T>) -> Response<http_body_util::combinators::UnsyncBoxBody<Bytes, hyper::Error>>
    where
        T: Body + Send + 'static,
        T::Data: Send,
        T::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        // Parse streamable HTTP context from request
        let context = StreamableHttpContext::from_request(&req);

        info!(
            "STREAMABLE HANDLER ENTRY: method={}, protocol={}, session={:?}, accepts_stream_frames={}, wants_sse_stream={}",
            req.method(),
            context.protocol_version.as_str(),
            context.session_id,
            context.accepts_stream_frames,
            context.wants_sse_stream()
        );

        // Validate request
        if let Err(error) = context.validate() {
            warn!("Invalid streamable HTTP request: {}", error);
            return StreamableResponse::Error {
                status: StatusCode::BAD_REQUEST,
                message: error,
            }
            .into_boxed_response(&context);
        }

        // Route based on MCP 2025-06-18 specification
        match req.method() {
            &Method::POST => {
                // ALL client messages (requests, notifications, responses) come via POST
                // Server decides whether to respond with JSON or SSE stream
                self.handle_client_message(req, context).await
            }
            &Method::GET => {
                // Optional SSE stream for server-initiated messages
                self.handle_sse_stream(req, context).await
            }
            &Method::DELETE => {
                // Optional session cleanup
                self.handle_session_delete(req, context).await
            }
            _ => StreamableResponse::Error {
                status: StatusCode::METHOD_NOT_ALLOWED,
                message: "Method not allowed for this endpoint".to_string(),
            }
            .into_boxed_response(&context),
        }
    }

    /// Handle GET request for optional SSE stream (MCP 2025-06-18 compliant)
    async fn handle_sse_stream<T>(
        &self,
        req: Request<T>,
        context: StreamableHttpContext,
    ) -> Response<http_body_util::combinators::UnsyncBoxBody<Bytes, hyper::Error>>
    where
        T: Body + Send + 'static,
    {
        info!(
            "Opening streaming connection for session: {:?}",
            context.session_id
        );

        // 1. Validate session exists and is authorized
        let session_id = match context.session_id {
            Some(ref id) => id.clone(),
            None => {
                warn!("Missing session ID for streaming GET request");
                return StreamableResponse::Error {
                    status: StatusCode::BAD_REQUEST,
                    message: "Mcp-Session-Id header required for streaming connection".to_string(),
                }
                .into_boxed_response(&context);
            }
        };

        // Validate session exists (do NOT create if missing)
        match self.validate_session_exists(&session_id).await {
            Ok(_) => {
                debug!("Session validation successful for streaming GET: {}", session_id);
            }
            Err(err) => {
                error!("Session validation failed for streaming GET {}: {}", session_id, err);
                return StreamableResponse::Error {
                    status: StatusCode::UNAUTHORIZED,
                    message: format!("Session validation failed: {}", err),
                }
                .into_boxed_response(&context);
            }
        }

        // 2. Create bi-directional stream with chunked transfer encoding
        // For MCP 2025-06-18 Streamable HTTP, we create a stream that can handle bidirectional JSON-RPC
        // Unlike SSE which is unidirectional server->client, this supports client->server and server->client

        // Extract Last-Event-ID for resumability (if client supports it)
        let last_event_id = req
            .headers()
            .get("Last-Event-ID")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());

        // Generate unique connection ID for tracking this stream
        let connection_id = uuid::Uuid::now_v7().to_string();

        debug!(
            "Creating streamable HTTP connection: session={}, connection={}, last_event_id={:?}",
            session_id, connection_id, last_event_id
        );

        // 3. Return streaming response supporting progressive message delivery
        // ✅ FIXED: Return the actual streaming response from StreamManager
        // This preserves event replay, resumability, and live streaming capabilities
        match self
            .stream_manager
            .handle_sse_connection(session_id.clone(), connection_id.clone(), last_event_id)
            .await
        {
            Ok(mut streaming_response) => {
                info!(
                    "✅ Streamable HTTP connection established: session={}, connection={}",
                    session_id, connection_id
                );

                // Merge MCP headers from context.response_headers()
                let mcp_headers = context.response_headers();
                for (key, value) in mcp_headers.iter() {
                    streaming_response.headers_mut().insert(key, value.clone());
                }

                // ✅ PRESERVE STREAMING: Return the streaming response with MCP headers
                // This maintains event replay from session storage and live streaming
                streaming_response
            }
            Err(err) => {
                error!("Failed to create streamable HTTP connection: {}", err);
                StreamableResponse::Error {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    message: format!("Streaming connection failed: {}", err),
                }
                .into_boxed_response(&context)
            }
        }
    }

    /// Validate that a session exists - do NOT create if missing
    async fn validate_session_exists(&self, session_id: &str) -> std::result::Result<(), String> {
        match self.session_storage.get_session(session_id).await {
            Ok(Some(_)) => {
                debug!("Session validation successful: {}", session_id);
                Ok(())
            }
            Ok(None) => {
                error!("Session not found: {}", session_id);
                Err(format!(
                    "Session '{}' not found. Sessions must be created via initialize request first.",
                    session_id
                ))
            }
            Err(err) => {
                error!("Failed to validate session {}: {}", session_id, err);
                Err(format!("Session validation failed: {}", err))
            }
        }
    }

    /// Handle POST request with streaming response
    #[allow(dead_code)]
    async fn handle_streaming_post<T>(
        &self,
        req: Request<T>,
        context: StreamableHttpContext,
    ) -> Response<http_body_util::combinators::UnsyncBoxBody<Bytes, hyper::Error>>
    where
        T: Body + Send + 'static,
    {
        info!(
            "Handling streaming POST for session: {:?}",
            context.session_id
        );

        // 1. Validate session exists and is authorized
        let session_id = match context.session_id {
            Some(ref id) => id.clone(),
            None => {
                warn!("Missing session ID for streaming POST request");
                return StreamableResponse::Error {
                    status: StatusCode::BAD_REQUEST,
                    message: "Mcp-Session-Id header required for streaming POST".to_string(),
                }
                .into_boxed_response(&context);
            }
        };

        // Validate session exists (do NOT create if missing)
        if let Err(err) = self.validate_session_exists(&session_id).await {
            error!("Session validation failed for streaming POST {}: {}", session_id, err);
            return StreamableResponse::Error {
                status: StatusCode::UNAUTHORIZED,
                message: format!("Session validation failed: {}", err),
            }
            .into_boxed_response(&context);
        }

        // Check content type
        let content_type = req
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or("");

        if !content_type.starts_with("application/json") {
            warn!("Invalid content type for streaming POST: {}", content_type);
            return StreamableResponse::Error {
                status: StatusCode::BAD_REQUEST,
                message: "Content-Type must be application/json".to_string(),
            }
            .into_boxed_response(&context);
        }

        // 2. Parse JSON-RPC request(s) from chunked request body
        let body_bytes = match req.into_body().collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(_err) => {
                error!("Failed to read streaming POST request body");
                return StreamableResponse::Error {
                    status: StatusCode::BAD_REQUEST,
                    message: "Failed to read request body".to_string(),
                }
                .into_boxed_response(&context);
            }
        };

        // Check body size
        if body_bytes.len() > self.config.max_body_size {
            warn!("Streaming POST request body too large: {} bytes", body_bytes.len());
            return StreamableResponse::Error {
                status: StatusCode::PAYLOAD_TOO_LARGE,
                message: "Request body too large".to_string(),
            }
            .into_boxed_response(&context);
        }

        // Parse as UTF-8
        let body_str = match std::str::from_utf8(&body_bytes) {
            Ok(s) => s,
            Err(err) => {
                error!("Invalid UTF-8 in streaming POST request body: {}", err);
                return StreamableResponse::Error {
                    status: StatusCode::BAD_REQUEST,
                    message: "Request body must be valid UTF-8".to_string(),
                }
                .into_boxed_response(&context);
            }
        };

        debug!("Received streaming POST JSON-RPC request: {}", body_str);

        // Parse JSON-RPC message
        use turul_mcp_json_rpc_server::dispatch::{parse_json_rpc_message, JsonRpcMessage, JsonRpcMessageResult};
        use turul_mcp_json_rpc_server::r#async::SessionContext;
        use crate::notification_bridge::StreamManagerNotificationBroadcaster;

        let message = match parse_json_rpc_message(body_str) {
            Ok(msg) => msg,
            Err(rpc_err) => {
                error!("JSON-RPC parse error in streaming POST: {}", rpc_err);
                let error_json = serde_json::to_string(&rpc_err).unwrap_or_else(|_| "{}".to_string());
                return Response::builder()
                    .status(StatusCode::OK) // JSON-RPC parse errors still use 200 OK
                    .header(CONTENT_TYPE, "application/json")
                    .header("MCP-Protocol-Version", context.protocol_version.as_str())
                    .body(Full::new(Bytes::from(error_json)))
                    .unwrap()
                    .map(|body| body.map_err(|never| match never {}).boxed_unsync())
                    .map(|body| body.map_err(|never| match never {}).boxed_unsync());
            }
        };

        // 3. Process via dispatcher with session context and streaming capabilities
        let broadcaster = Arc::new(StreamManagerNotificationBroadcaster::new(Arc::clone(
            &self.stream_manager,
        )));
        let broadcaster_any = Arc::new(broadcaster) as Arc<dyn std::any::Any + Send + Sync>;

        let session_context = SessionContext {
            session_id: session_id.clone(),
            metadata: std::collections::HashMap::new(),
            broadcaster: Some(broadcaster_any),
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
        };

        let message_result = match message {
            JsonRpcMessage::Request(request) => {
                debug!("Processing streaming POST JSON-RPC request: method={}", request.method);
                let response = self
                    .dispatcher
                    .handle_request_with_context(request, session_context)
                    .await;

                // Convert JsonRpcMessage to JsonRpcMessageResult
                match response {
                    turul_mcp_json_rpc_server::JsonRpcMessage::Response(resp) => {
                        JsonRpcMessageResult::Response(resp)
                    }
                    turul_mcp_json_rpc_server::JsonRpcMessage::Error(err) => {
                        JsonRpcMessageResult::Error(err)
                    }
                }
            }
            JsonRpcMessage::Notification(notification) => {
                debug!(
                    "Processing streaming POST JSON-RPC notification: method={}",
                    notification.method
                );

                let result = self
                    .dispatcher
                    .handle_notification_with_context(notification, Some(session_context))
                    .await;

                if let Err(err) = result {
                    error!("Streaming POST notification handling error: {}", err);
                }
                JsonRpcMessageResult::NoResponse
            }
        };

        // 4. Stream responses back with progressive message delivery
        // For now, return the response immediately - TODO: implement actual streaming
        match message_result {
            JsonRpcMessageResult::Response(response) => {
                let response_json = serde_json::to_string(&response)
                    .unwrap_or_else(|_| r#"{"error": "Failed to serialize response"}"#.to_string());

                Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, "application/json")
                    .header("MCP-Protocol-Version", context.protocol_version.as_str())
                    .header("Mcp-Session-Id", &session_id)
                    .body(Full::new(Bytes::from(response_json)))
                    .unwrap()
                    .map(|body| body.map_err(|never| match never {}).boxed_unsync())
                    .map(|body| body.map_err(|never| match never {}).boxed_unsync())
            }
            JsonRpcMessageResult::Error(error) => {
                let error_json = serde_json::to_string(&error)
                    .unwrap_or_else(|_| r#"{"error": "Internal error"}"#.to_string());

                Response::builder()
                    .status(StatusCode::OK) // JSON-RPC errors still return 200 OK
                    .header(CONTENT_TYPE, "application/json")
                    .header("MCP-Protocol-Version", context.protocol_version.as_str())
                    .header("Mcp-Session-Id", &session_id)
                    .body(Full::new(Bytes::from(error_json)))
                    .unwrap()
                    .map(|body| body.map_err(|never| match never {}).boxed_unsync())
                    .map(|body| body.map_err(|never| match never {}).boxed_unsync())
            }
            JsonRpcMessageResult::NoResponse => {
                // Notifications return 202 Accepted per MCP spec
                Response::builder()
                    .status(StatusCode::ACCEPTED)
                    .header("MCP-Protocol-Version", context.protocol_version.as_str())
                    .header("Mcp-Session-Id", &session_id)
                    .body(Full::new(Bytes::new()))
                    .unwrap()
                    .map(|body| body.map_err(|never| match never {}).boxed_unsync())
                    .map(|body| body.map_err(|never| match never {}).boxed_unsync())
            }
        }
    }

    /// Handle POST request with JSON response (legacy compatibility)
    #[allow(dead_code)]
    async fn handle_json_post<T>(
        &self,
        req: Request<T>,
        context: StreamableHttpContext,
    ) -> Response<http_body_util::combinators::UnsyncBoxBody<Bytes, hyper::Error>>
    where
        T: Body + Send + 'static,
    {
        info!("Handling JSON POST (non-streaming/legacy)");

        // 1. Parse JSON-RPC request(s) from request body (legacy clients don't require sessions)

        // Check content type
        let content_type = req
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or("");

        if !content_type.starts_with("application/json") {
            warn!("Invalid content type for legacy POST: {}", content_type);
            return StreamableResponse::Error {
                status: StatusCode::BAD_REQUEST,
                message: "Content-Type must be application/json".to_string(),
            }
            .into_boxed_response(&context);
        }

        // Read request body
        let body_bytes = match req.into_body().collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(_err) => {
                error!("Failed to read legacy POST request body");
                return StreamableResponse::Error {
                    status: StatusCode::BAD_REQUEST,
                    message: "Failed to read request body".to_string(),
                }
                .into_boxed_response(&context);
            }
        };

        // Check body size
        if body_bytes.len() > self.config.max_body_size {
            warn!("Legacy POST request body too large: {} bytes", body_bytes.len());
            return StreamableResponse::Error {
                status: StatusCode::PAYLOAD_TOO_LARGE,
                message: "Request body too large".to_string(),
            }
            .into_boxed_response(&context);
        }

        // Parse as UTF-8
        let body_str = match std::str::from_utf8(&body_bytes) {
            Ok(s) => s,
            Err(err) => {
                error!("Invalid UTF-8 in legacy POST request body: {}", err);
                return StreamableResponse::Error {
                    status: StatusCode::BAD_REQUEST,
                    message: "Request body must be valid UTF-8".to_string(),
                }
                .into_boxed_response(&context);
            }
        };

        debug!("Received legacy POST JSON-RPC request: {}", body_str);

        // Parse JSON-RPC message
        use turul_mcp_json_rpc_server::dispatch::{parse_json_rpc_message, JsonRpcMessage, JsonRpcMessageResult};

        let message = match parse_json_rpc_message(body_str) {
            Ok(msg) => msg,
            Err(rpc_err) => {
                error!("JSON-RPC parse error in legacy POST: {}", rpc_err);
                let error_json = serde_json::to_string(&rpc_err).unwrap_or_else(|_| "{}".to_string());
                return Response::builder()
                    .status(StatusCode::OK) // JSON-RPC parse errors still use 200 OK
                    .header(CONTENT_TYPE, "application/json")
                    .header("MCP-Protocol-Version", context.protocol_version.as_str())
                    .body(Full::new(Bytes::from(error_json)))
                    .unwrap()
                    .map(|body| body.map_err(|never| match never {}).boxed_unsync());
            }
        };

        // 2. Process via dispatcher (no session context for legacy clients)
        // Legacy clients (MCP 2024-11-05) don't use sessions, so no session context
        let message_result = match message {
            JsonRpcMessage::Request(request) => {
                debug!("Processing legacy POST JSON-RPC request: method={}", request.method);

                // Special handling for initialize requests - legacy clients can create sessions too
                let response = if request.method == "initialize" {
                    debug!("Handling legacy initialize request - creating new session");

                    // Let session storage create the session and generate the ID
                    use turul_mcp_protocol::ServerCapabilities;
                    match self.session_storage.create_session(ServerCapabilities::default()).await {
                        Ok(session_info) => {
                            debug!("Created new session for legacy client: {}", session_info.session_id);

                            // Create session context for initialize response
                            use turul_mcp_json_rpc_server::r#async::SessionContext;
                            use crate::notification_bridge::StreamManagerNotificationBroadcaster;

                            let broadcaster = Arc::new(StreamManagerNotificationBroadcaster::new(Arc::clone(&self.stream_manager)));
                            let broadcaster_any = Arc::new(broadcaster) as Arc<dyn std::any::Any + Send + Sync>;

                            let session_context = SessionContext {
                                session_id: session_info.session_id.clone(),
                                metadata: std::collections::HashMap::new(),
                                broadcaster: Some(broadcaster_any),
                                timestamp: chrono::Utc::now().timestamp_millis() as u64,
                            };

                            self.dispatcher
                                .handle_request_with_context(request, session_context)
                                .await
                        }
                        Err(err) => {
                            error!("Failed to create session during legacy initialize: {}", err);
                            let error_msg = format!("Session creation failed: {}", err);
                            turul_mcp_json_rpc_server::JsonRpcMessage::error(
                                turul_mcp_json_rpc_server::JsonRpcError::internal_error(
                                    Some(request.id),
                                    Some(error_msg),
                                ),
                            )
                        }
                    }
                } else {
                    // For non-initialize requests, process without session context (legacy mode)
                    self.dispatcher.handle_request(request).await
                };

                // Convert JsonRpcMessage to JsonRpcMessageResult
                match response {
                    turul_mcp_json_rpc_server::JsonRpcMessage::Response(resp) => {
                        JsonRpcMessageResult::Response(resp)
                    }
                    turul_mcp_json_rpc_server::JsonRpcMessage::Error(err) => {
                        JsonRpcMessageResult::Error(err)
                    }
                }
            }
            JsonRpcMessage::Notification(notification) => {
                debug!("Processing legacy POST JSON-RPC notification: method={}", notification.method);

                // Process notification without session context (legacy mode)
                let result = self
                    .dispatcher
                    .handle_notification_with_context(notification, None)
                    .await;

                if let Err(err) = result {
                    error!("Legacy POST notification handling error: {}", err);
                }
                JsonRpcMessageResult::NoResponse
            }
        };

        // 3. Return single JSON response (no streaming) - legacy compatibility
        match message_result {
            JsonRpcMessageResult::Response(response) => {
                let response_json = serde_json::to_string(&response)
                    .unwrap_or_else(|_| r#"{"error": "Failed to serialize response"}"#.to_string());

                Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, "application/json")
                    .header("MCP-Protocol-Version", context.protocol_version.as_str())
                    .body(Full::new(Bytes::from(response_json)))
                    .unwrap()
                    .map(|body| body.map_err(|never| match never {}).boxed_unsync())
            }
            JsonRpcMessageResult::Error(error) => {
                let error_json = serde_json::to_string(&error)
                    .unwrap_or_else(|_| r#"{"error": "Internal error"}"#.to_string());

                Response::builder()
                    .status(StatusCode::OK) // JSON-RPC errors still return 200 OK
                    .header(CONTENT_TYPE, "application/json")
                    .header("MCP-Protocol-Version", context.protocol_version.as_str())
                    .body(Full::new(Bytes::from(error_json)))
                    .unwrap()
                    .map(|body| body.map_err(|never| match never {}).boxed_unsync())
            }
            JsonRpcMessageResult::NoResponse => {
                // Notifications return 202 Accepted per MCP spec
                Response::builder()
                    .status(StatusCode::ACCEPTED)
                    .header("MCP-Protocol-Version", context.protocol_version.as_str())
                    .body(Full::new(Bytes::new()))
                    .unwrap()
                    .map(|body| body.map_err(|never| match never {}).boxed_unsync())
            }
        }
    }

    /// Handle DELETE request for session cleanup
    async fn handle_session_delete<T>(
        &self,
        _req: Request<T>,
        context: StreamableHttpContext,
    ) -> Response<http_body_util::combinators::UnsyncBoxBody<Bytes, hyper::Error>>
    where
        T: Body + Send + 'static,
    {
        if let Some(session_id) = &context.session_id {
            info!("Deleting session: {}", session_id);

            // Implement proper session cleanup for Streamable HTTP
            // 1. Close any active streaming connections for this session
            let closed_connections = self
                .stream_manager
                .close_session_connections(session_id)
                .await;
            debug!(
                "Closed {} streaming connections for session: {}",
                closed_connections, session_id
            );

            // 2. Mark session as terminated instead of immediate deletion (for proper lifecycle management)
            match self.session_storage.get_session(session_id).await {
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

                    // 3. Update session with termination markers
                    match self.session_storage.update_session(session_info).await {
                        Ok(()) => {
                            info!(
                                "Session {} marked as terminated (TTL will handle cleanup)",
                                session_id
                            );

                            // Return success response with proper headers
                            Response::builder()
                                .status(StatusCode::OK)
                                .header(CONTENT_TYPE, "application/json")
                                .header("MCP-Protocol-Version", context.protocol_version.as_str())
                                .header("Mcp-Session-Id", session_id)
                                .body(Full::new(Bytes::from(serde_json::to_string(&serde_json::json!({
                                    "status": "session_terminated",
                                    "session_id": session_id,
                                    "closed_connections": closed_connections,
                                    "message": "Session marked for cleanup"
                                })).unwrap_or_else(|_| r#"{"status":"session_terminated"}"#.to_string()))))
                                .unwrap()
                    .map(|body| body.map_err(|never| match never {}).boxed_unsync())
                        }
                        Err(err) => {
                            error!(
                                "Error marking session {} as terminated: {}",
                                session_id, err
                            );
                            // Fallback to deletion if update fails
                            match self.session_storage.delete_session(session_id).await {
                                Ok(_) => {
                                    info!("Session {} deleted as fallback", session_id);
                                    Response::builder()
                                        .status(StatusCode::OK)
                                        .header(CONTENT_TYPE, "application/json")
                                        .header("MCP-Protocol-Version", context.protocol_version.as_str())
                                        .body(Full::new(Bytes::from(serde_json::to_string(&serde_json::json!({
                                            "status": "session_deleted",
                                            "session_id": session_id,
                                            "closed_connections": closed_connections,
                                            "message": "Session removed"
                                        })).unwrap_or_else(|_| r#"{"status":"session_deleted"}"#.to_string()))))
                                        .unwrap()
                    .map(|body| body.map_err(|never| match never {}).boxed_unsync())
                                }
                                Err(delete_err) => {
                                    error!(
                                        "Error deleting session {} as fallback: {}",
                                        session_id, delete_err
                                    );
                                    StreamableResponse::Error {
                                        status: StatusCode::INTERNAL_SERVER_ERROR,
                                        message: "Session termination error".to_string(),
                                    }
                                    .into_boxed_response(&context)
                                }
                            }
                        }
                    }
                }
                Ok(None) => {
                    // Session not found
                    Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .header(CONTENT_TYPE, "application/json")
                        .header("MCP-Protocol-Version", context.protocol_version.as_str())
                        .body(Full::new(Bytes::from(serde_json::to_string(&serde_json::json!({
                            "status": "session_not_found",
                            "session_id": session_id,
                            "message": "Session not found"
                        })).unwrap_or_else(|_| r#"{"status":"session_not_found"}"#.to_string()))))
                        .unwrap()
                    .map(|body| body.map_err(|never| match never {}).boxed_unsync())
                }
                Err(err) => {
                    error!(
                        "Error retrieving session {} for termination: {}",
                        session_id, err
                    );
                    StreamableResponse::Error {
                        status: StatusCode::INTERNAL_SERVER_ERROR,
                        message: "Session lookup error".to_string(),
                    }
                    .into_boxed_response(&context)
                }
            }
        } else {
            StreamableResponse::Error {
                status: StatusCode::BAD_REQUEST,
                message: "Mcp-Session-Id header required for session deletion".to_string(),
            }
            .into_boxed_response(&context)
        }
    }

    /// Handle POST with TRUE streaming using hyper::Body::channel()
    /// This implements actual MCP 2025-06-18 chunked transfer encoding
    async fn handle_streaming_post_real<T>(
        &self,
        req: Request<T>,
        mut context: StreamableHttpContext,
    ) -> Response<http_body_util::combinators::UnsyncBoxBody<Bytes, hyper::Error>>
    where
        T: Body + Send + 'static,
    {
        info!("🚀🚀🚀 STREAMING HANDLER CALLED - Using TRUE streaming POST");

        // Parse request body (still need to collect for JSON-RPC parsing)
        let body_bytes = match req.into_body().collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(_err) => {
                error!("Failed to read streaming POST request body");
                return StreamableResponse::Error {
                    status: StatusCode::BAD_REQUEST,
                    message: "Failed to read request body".to_string(),
                }
                .into_boxed_response(&context);
            }
        };

        // Check body size
        if body_bytes.len() > self.config.max_body_size {
            warn!("Streaming POST request body too large: {} bytes", body_bytes.len());
            return StreamableResponse::Error {
                status: StatusCode::PAYLOAD_TOO_LARGE,
                message: "Request body too large".to_string(),
            }
            .into_boxed_response(&context);
        }

        // Parse as UTF-8
        let body_str = match std::str::from_utf8(&body_bytes) {
            Ok(s) => s,
            Err(err) => {
                error!("Invalid UTF-8 in streaming POST request body: {}", err);
                return StreamableResponse::Error {
                    status: StatusCode::BAD_REQUEST,
                    message: "Request body must be valid UTF-8".to_string(),
                }
                .into_boxed_response(&context);
            }
        };

        debug!("🚀 Streaming POST received JSON-RPC request: {}", body_str);

        // Parse JSON-RPC message
        use turul_mcp_json_rpc_server::dispatch::{parse_json_rpc_message, JsonRpcMessage};
        use turul_mcp_json_rpc_server::error::JsonRpcErrorObject;

        let message = match parse_json_rpc_message(body_str) {
            Ok(msg) => msg,
            Err(rpc_err) => {
                error!("JSON-RPC parse error in streaming POST: {}", rpc_err);
                let error_json = serde_json::to_string(&rpc_err).unwrap_or_else(|_| "{}".to_string());

                // Return error with MCP headers (no session header for parse errors)
                return Response::builder()
                    .status(StatusCode::OK) // JSON-RPC parse errors still use 200 OK
                    .header(CONTENT_TYPE, "application/json")
                    .header("MCP-Protocol-Version", context.protocol_version.as_str())
                    .body(Full::new(Bytes::from(error_json)).map_err(|never| match never {}).boxed_unsync())
                    .unwrap();
            }
        };

        // Validate session requirements based on method
        let session_id = match &message {
            JsonRpcMessage::Request(req) if req.method == "initialize" => {
                // Initialize can create session if none exists
                if let Some(existing_id) = &context.session_id {
                    // Validate existing session for initialize
                    if let Err(err) = self.validate_session_exists(existing_id).await {
                        warn!("Invalid session ID {} during initialize: {}", existing_id, err);
                        return StreamableResponse::Error {
                            status: StatusCode::UNAUTHORIZED,
                            message: "Invalid or expired session".to_string(),
                        }
                        .into_boxed_response(&context);
                    }
                    existing_id.clone()
                } else {
                    // Create new session for initialize
                    match self.session_storage.create_session(turul_mcp_protocol::ServerCapabilities::default()).await {
                        Ok(session_info) => {
                            debug!("Created new session for initialize: {}", session_info.session_id);
                            context.session_id = Some(session_info.session_id.clone());
                            session_info.session_id
                        }
                        Err(err) => {
                            error!("Failed to create session during initialize: {}", err);
                            return StreamableResponse::Error {
                                status: StatusCode::INTERNAL_SERVER_ERROR,
                                message: "Failed to create session".to_string(),
                            }
                            .into_boxed_response(&context);
                        }
                    }
                }
            }
            JsonRpcMessage::Request(_) | JsonRpcMessage::Notification(_) => {
                // All other methods REQUIRE session ID
                if let Some(existing_id) = &context.session_id {
                    // Validate existing session
                    if let Err(err) = self.validate_session_exists(existing_id).await {
                        warn!("Invalid session ID {}: {}", existing_id, err);
                        return StreamableResponse::Error {
                            status: StatusCode::UNAUTHORIZED,
                            message: "Invalid or expired session".to_string(),
                        }
                        .into_boxed_response(&context);
                    }
                    existing_id.clone()
                } else {
                    // Return 401 JSON-RPC error for missing session
                    let method_name = match &message {
                        JsonRpcMessage::Request(req) => &req.method,
                        JsonRpcMessage::Notification(notif) => &notif.method,
                    };
                    let request_id = match &message {
                        JsonRpcMessage::Request(req) => Some(req.id.clone()),
                        JsonRpcMessage::Notification(_) => None,
                    };

                    warn!("Missing session ID for method: {}", method_name);

                    let error_response = turul_mcp_json_rpc_server::JsonRpcError::new(
                        request_id,
                        JsonRpcErrorObject::server_error(
                            -32001,
                            "Missing Mcp-Session-Id header. Call initialize first.",
                            None::<serde_json::Value>
                        )
                    );

                    let error_json = serde_json::to_string(&error_response).unwrap_or_else(|_| "{}".to_string());

                    return Response::builder()
                        .status(StatusCode::UNAUTHORIZED)
                        .header(CONTENT_TYPE, "application/json")
                        .header("MCP-Protocol-Version", context.protocol_version.as_str())
                        .body(Full::new(Bytes::from(error_json)).map_err(|never| match never {}).boxed_unsync())
                        .unwrap();
                }
            }
        };

        info!("Processing streaming request with session: {}", session_id);

        // Create streaming response using hyper::Body::channel()
        match message {
            JsonRpcMessage::Request(request) => {
                debug!("🚀 Processing streaming JSON-RPC request: method={}", request.method);
                self.create_streaming_response(request, session_id, context).await
            }
            JsonRpcMessage::Notification(notification) => {
                debug!("🚀 Processing streaming JSON-RPC notification: method={}", notification.method);
                // Handle notification (they don't return responses)
                // TODO: Process notification through dispatcher

                // Return 202 Accepted with MCP headers
                Response::builder()
                    .status(StatusCode::ACCEPTED)
                    .header("MCP-Protocol-Version", context.protocol_version.as_str())
                    .header("Mcp-Session-Id", &session_id)
                    .body(Full::new(Bytes::new()).map_err(|never| match never {}).boxed_unsync())
                    .unwrap()
            }
        }
    }

    /// Create a streaming response using hyper::Body::channel()
    /// This enables true progressive responses with Transfer-Encoding: chunked
    async fn create_streaming_response(
        &self,
        request: turul_mcp_json_rpc_server::JsonRpcRequest,
        session_id: String,
        context: StreamableHttpContext,
    ) -> Response<http_body_util::combinators::UnsyncBoxBody<Bytes, hyper::Error>> {
        // Create channel for streaming response
        use http_body_util::StreamBody;
        use tokio_stream::wrappers::UnboundedReceiverStream;
        use tokio_stream::StreamExt; // Add StreamExt for map method

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Result<bytes::Bytes, hyper::Error>>();
        let body_stream = UnboundedReceiverStream::new(rx).map(|item| {
            item.map(|bytes| http_body::Frame::data(bytes))
        });
        let body = StreamBody::new(body_stream);

        // Create session context with notification broadcaster
        use crate::notification_bridge::StreamManagerNotificationBroadcaster;
        use turul_mcp_json_rpc_server::SessionContext;

        let broadcaster = Arc::new(StreamManagerNotificationBroadcaster::new(Arc::clone(&self.stream_manager)));
        let broadcaster_any = Arc::new(broadcaster) as Arc<dyn std::any::Any + Send + Sync>;

        let session_context = SessionContext {
            session_id: session_id.clone(),
            metadata: std::collections::HashMap::new(),
            broadcaster: Some(broadcaster_any),
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
        };

        // Spawn task to handle streaming dispatch
        let dispatcher = Arc::clone(&self.dispatcher);
        let request_id = request.id.clone();
        let sender = tx; // Rename for clarity

        tokio::spawn(async move {
            // TODO: Use StreamingJsonRpcDispatcher when available
            // For now, use regular dispatcher and simulate streaming

            debug!("🚀 Spawning streaming task for request ID: {:?}", request_id);

            // Simulate progress token (in real implementation, this would come from tools)
            let progress_frame = turul_mcp_json_rpc_server::JsonRpcFrame::Progress {
                request_id: request_id.clone(),
                progress: serde_json::json!({
                    "status": "processing",
                    "percentage": 50
                }),
                progress_token: Some(format!("token_{}", uuid::Uuid::now_v7())),
            };

            let progress_json = progress_frame.to_json();
            let progress_chunk = format!("{}\n", serde_json::to_string(&progress_json).unwrap());

            if let Err(err) = sender.send(Ok(Bytes::from(progress_chunk))) {
                error!("Failed to send progress chunk: {}", err);
                return;
            }

            // Process actual request
            let response = dispatcher.handle_request_with_context(request, session_context).await;

            // Convert to streaming frame
            let final_frame = match response {
                turul_mcp_json_rpc_server::JsonRpcMessage::Response(resp) => {
                    turul_mcp_json_rpc_server::JsonRpcFrame::FinalResult {
                        request_id: request_id.clone(),
                        result: match resp.result {
                            turul_mcp_json_rpc_server::response::ResponseResult::Success(val) => val,
                            turul_mcp_json_rpc_server::response::ResponseResult::Null => serde_json::Value::Null,
                        },
                    }
                }
                turul_mcp_json_rpc_server::JsonRpcMessage::Error(err) => {
                    turul_mcp_json_rpc_server::JsonRpcFrame::Error {
                        request_id: request_id.clone(),
                        error: turul_mcp_json_rpc_server::error::JsonRpcErrorObject {
                            code: err.error.code,
                            message: err.error.message,
                            data: err.error.data,
                        },
                    }
                }
            };

            let final_json = final_frame.to_json();
            let final_chunk = format!("{}\n", serde_json::to_string(&final_json).unwrap());

            if let Err(err) = sender.send(Ok(Bytes::from(final_chunk))) {
                error!("Failed to send final chunk: {}", err);
            }

            debug!("🚀 Streaming task completed for request ID: {:?}", request_id);
        });

        // Build response with MCP headers merged from context
        let mut response = Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "application/json")
            .header("Transfer-Encoding", "chunked") // Key: Enable chunked encoding!
            .header("Cache-Control", "no-cache")
            .body(http_body_util::BodyExt::boxed_unsync(body))
            .unwrap();

        // Merge MCP headers from context.response_headers()
        let mcp_headers = context.response_headers();
        for (key, value) in mcp_headers.iter() {
            response.headers_mut().insert(key, value.clone());
        }

        response
    }

    /// Handle POST with buffered response (fallback for legacy clients)
    #[allow(dead_code)]
    async fn handle_buffered_post<T>(
        &self,
        _req: Request<T>,
        context: StreamableHttpContext,
        session_id: String,
    ) -> Response<http_body_util::combinators::UnsyncBoxBody<Bytes, hyper::Error>>
    where
        T: Body + Send + 'static,
    {
        info!("Using buffered POST for legacy client, session: {}", session_id);

        // Use the existing logic (simplified version)
        // TODO: Extract common logic into helper method

        Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "application/json")
            .header("MCP-Protocol-Version", context.protocol_version.as_str())
            .header("Mcp-Session-Id", &session_id)
            .body(Full::new(Bytes::from(r#"{"jsonrpc":"2.0","id":1,"result":"buffered"}"#)).map_err(|never| match never {}).boxed_unsync())
            .unwrap()
    }

    /// Handle POST request - unified handler for all client messages (MCP 2025-06-18 compliant)
    /// Processes JSON-RPC requests, notifications, and responses
    /// Server decides whether to respond with JSON or SSE stream based on message type
    async fn handle_client_message<T>(
        &self,
        req: Request<T>,
        context: StreamableHttpContext,
    ) -> Response<http_body_util::combinators::UnsyncBoxBody<Bytes, hyper::Error>>
    where
        T: Body + Send + 'static,
    {
        info!("Handling client message via POST (MCP 2025-06-18)");

        // Reject POST if accepts_stream_frames is false
        // Per MCP spec: "Include Accept header with application/json and text/event-stream"
        if !context.accepts_stream_frames {
            warn!("Client POST missing application/json in Accept header");
            return StreamableResponse::Error {
                status: StatusCode::BAD_REQUEST,
                message: "Accept header must include application/json".to_string(),
            }
            .into_boxed_response(&context);
        }

        // Check content type
        let content_type = req
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or("");
        if !content_type.starts_with("application/json") {
            warn!("Invalid content type for POST: {}", content_type);
            return StreamableResponse::Error {
                status: StatusCode::BAD_REQUEST,
                message: "Content-Type must be application/json".to_string(),
            }
            .into_boxed_response(&context);
        }

        // 🚀 DECIDE: Use streaming for ALL POST requests per MCP 2025-06-18
        // MCP spec requires chunked responses for progressive results, even with Accept: application/json

        // Session validation now happens in handle_streaming_post_real after parsing the method
        info!("Using chunked streaming for POST request");
        return self.handle_streaming_post_real(req, context).await;

        // DEAD CODE: This buffered logic is unreachable due to early return above
        // TODO: Remove or implement as proper fallback when streaming is not available
        /*
        // Read and parse request body
        let body_bytes = match req.into_body().collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(_err) => {
                error!("Failed to read POST request body");
                return StreamableResponse::Error {
                    status: StatusCode::BAD_REQUEST,
                    message: "Failed to read request body".to_string(),
                }
                .into_boxed_response(&context);
            }
        };

        // Check body size
        if body_bytes.len() > self.config.max_body_size {
            warn!("POST request body too large: {} bytes", body_bytes.len());
            return StreamableResponse::Error {
                status: StatusCode::PAYLOAD_TOO_LARGE,
                message: "Request body too large".to_string(),
            }
            .into_boxed_response(&context);
        }

        // Parse as UTF-8
        let body_str = match std::str::from_utf8(&body_bytes) {
            Ok(s) => s,
            Err(err) => {
                error!("Invalid UTF-8 in POST request body: {}", err);
                return StreamableResponse::Error {
                    status: StatusCode::BAD_REQUEST,
                    message: "Request body must be valid UTF-8".to_string(),
                }
                .into_boxed_response(&context);
            }
        };

        debug!("Received client message: {}", body_str);

        // Parse JSON-RPC message
        use turul_mcp_json_rpc_server::dispatch::{parse_json_rpc_message, JsonRpcMessage, JsonRpcMessageResult};
        use turul_mcp_json_rpc_server::r#async::SessionContext;
        use crate::notification_bridge::StreamManagerNotificationBroadcaster;

        let message = match parse_json_rpc_message(body_str) {
            Ok(msg) => msg,
            Err(rpc_err) => {
                error!("JSON-RPC parse error in POST: {}", rpc_err);
                let error_json = serde_json::to_string(&rpc_err).unwrap_or_else(|_| "{}".to_string());
                return Response::builder()
                    .status(StatusCode::OK) // JSON-RPC parse errors still use 200 OK
                    .header(CONTENT_TYPE, "application/json")
                    .header("MCP-Protocol-Version", context.protocol_version.as_str())
                    .body(Full::new(Bytes::from(error_json)))
                    .unwrap()
                    .map(|body| body.map_err(|never| match never {}).boxed_unsync());
            }
        };

        // Create session context if session ID provided (sessions are optional per MCP spec)
        let session_context = if let Some(ref session_id) = context.session_id {
            // Validate session exists (do NOT create if missing)
            if let Err(err) = self.validate_session_exists(session_id).await {
                error!("Session validation failed: {}", err);
                return StreamableResponse::Error {
                    status: StatusCode::UNAUTHORIZED,
                    message: format!("Session validation failed: {}", err),
                }
                .into_boxed_response(&context);
            }

            let broadcaster = Arc::new(StreamManagerNotificationBroadcaster::new(Arc::clone(&self.stream_manager)));
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

        // Process message based on type
        let message_result = match message {
            JsonRpcMessage::Request(request) => {
                debug!("Processing JSON-RPC request: method={}", request.method);

                // For requests, server decides: immediate JSON response or SSE stream
                // For now, always use immediate JSON response (TODO: implement streaming decision logic)
                let response = match session_context {
                    Some(ctx) => self.dispatcher.handle_request_with_context(request, ctx).await,
                    None => {
                        // Handle requests without session context
                        // Special case: initialize requests can create sessions
                        if request.method == "initialize" {
                            debug!("Handling initialize request - creating new session");
                            use turul_mcp_protocol::ServerCapabilities;
                            match self.session_storage.create_session(ServerCapabilities::default()).await {
                                Ok(session_info) => {
                                    debug!("Created new session: {}", session_info.session_id);
                                    let broadcaster = Arc::new(StreamManagerNotificationBroadcaster::new(Arc::clone(&self.stream_manager)));
                                    let broadcaster_any = Arc::new(broadcaster) as Arc<dyn std::any::Any + Send + Sync>;
                                    let new_session_context = SessionContext {
                                        session_id: session_info.session_id.clone(),
                                        metadata: std::collections::HashMap::new(),
                                        broadcaster: Some(broadcaster_any),
                                        timestamp: chrono::Utc::now().timestamp_millis() as u64,
                                    };
                                    self.dispatcher.handle_request_with_context(request, new_session_context).await
                                }
                                Err(err) => {
                                    error!("Failed to create session during initialize: {}", err);
                                    let error_msg = format!("Session creation failed: {}", err);
                                    turul_mcp_json_rpc_server::JsonRpcMessage::error(
                                        turul_mcp_json_rpc_server::JsonRpcError::internal_error(
                                            Some(request.id), Some(error_msg)
                                        )
                                    )
                                }
                            }
                        } else {
                            // Other requests without session context
                            warn!("Request without session context: {}", request.method);
                            turul_mcp_json_rpc_server::JsonRpcMessage::error(
                                turul_mcp_json_rpc_server::JsonRpcError::invalid_request(
                                    Some(request.id)
                                )
                            )
                        }
                    }
                };

                match response {
                    turul_mcp_json_rpc_server::JsonRpcMessage::Response(resp) => {
                        JsonRpcMessageResult::Response(resp)
                    }
                    turul_mcp_json_rpc_server::JsonRpcMessage::Error(err) => {
                        JsonRpcMessageResult::Error(err)
                    }
                }
            }
            JsonRpcMessage::Notification(notification) => {
                debug!("Processing JSON-RPC notification: method={}", notification.method);
                let result = match session_context {
                    Some(ctx) => self.dispatcher.handle_notification_with_context(notification, Some(ctx)).await,
                    None => self.dispatcher.handle_notification_with_context(notification, None).await,
                };
                if let Err(err) = result {
                    error!("Notification handling error: {}", err);
                }
                JsonRpcMessageResult::NoResponse
            }
        };

        // Return appropriate response based on message result
        match message_result {
            JsonRpcMessageResult::Response(response) => {
                // TODO: Here we could decide to return SSE stream instead of JSON for complex responses
                let response_json = serde_json::to_string(&response)
                    .unwrap_or_else(|_| r#"{"error": "Failed to serialize response"}"#.to_string());
                Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, "application/json")
                    .header("MCP-Protocol-Version", context.protocol_version.as_str())
                    .body(Full::new(Bytes::from(response_json)))
                    .unwrap()
                    .map(|body| body.map_err(|never| match never {}).boxed_unsync())
            }
            JsonRpcMessageResult::Error(error) => {
                let error_json = serde_json::to_string(&error)
                    .unwrap_or_else(|_| r#"{"error": "Internal error"}"#.to_string());
                Response::builder()
                    .status(StatusCode::OK) // JSON-RPC errors still return 200 OK
                    .header(CONTENT_TYPE, "application/json")
                    .header("MCP-Protocol-Version", context.protocol_version.as_str())
                    .body(Full::new(Bytes::from(error_json)))
                    .unwrap()
                    .map(|body| body.map_err(|never| match never {}).boxed_unsync())
            }
            JsonRpcMessageResult::NoResponse => {
                // Notifications return 202 Accepted per MCP spec
                Response::builder()
                    .status(StatusCode::ACCEPTED)
                    .header("MCP-Protocol-Version", context.protocol_version.as_str())
                    .body(Full::new(Bytes::new()))
                    .unwrap()
                    .map(|body| body.map_err(|never| match never {}).boxed_unsync())
            }
        }
        */
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_version_parsing() {
        assert_eq!(
            McpProtocolVersion::parse_version("2024-11-05"),
            Some(McpProtocolVersion::V2024_11_05)
        );
        assert_eq!(
            McpProtocolVersion::parse_version("2025-03-26"),
            Some(McpProtocolVersion::V2025_03_26)
        );
        assert_eq!(
            McpProtocolVersion::parse_version("2025-06-18"),
            Some(McpProtocolVersion::V2025_06_18)
        );
        assert_eq!(McpProtocolVersion::parse_version("invalid"), None);
    }

    #[test]
    fn test_version_capabilities() {
        let v1 = McpProtocolVersion::V2024_11_05;
        assert!(!v1.supports_streamable_http());
        assert!(!v1.supports_meta_fields());

        let v2 = McpProtocolVersion::V2025_03_26;
        assert!(v2.supports_streamable_http());
        assert!(!v2.supports_meta_fields());

        let v3 = McpProtocolVersion::V2025_06_18;
        assert!(v3.supports_streamable_http());
        assert!(v3.supports_meta_fields());
        assert!(v3.supports_cursors());
        assert!(v3.supports_progress_tokens());
        assert!(v3.supports_elicitation());
    }

    #[test]
    fn test_context_validation() {
        let mut context = StreamableHttpContext {
            protocol_version: McpProtocolVersion::V2025_06_18,
            session_id: Some("test-session".to_string()),
            wants_sse_stream: true,
            accepts_stream_frames: true,
            headers: HashMap::new(),
        };

        assert!(context.validate().is_ok());

        // Test invalid cases
        context.accepts_stream_frames = false;
        assert!(context.validate().is_err());

        context.accepts_stream_frames = true;
        context.protocol_version = McpProtocolVersion::V2024_11_05;
        context.wants_sse_stream = true;
        assert!(context.validate().is_err());

        context.protocol_version = McpProtocolVersion::V2025_06_18;
        context.session_id = None;
        assert!(context.validate().is_err());
    }
}
