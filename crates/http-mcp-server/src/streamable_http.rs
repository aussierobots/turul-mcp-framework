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
use std::sync::Arc;
use std::pin::Pin;

use hyper::{Request, Response, Method, StatusCode, HeaderMap};
use http_body_util::Full;
use bytes::Bytes;
use hyper::header::{CONTENT_TYPE, ACCEPT};
use tracing::{warn, info};
use futures::Stream;
use http_body::Body;
use serde_json::Value;

use crate::ServerConfig;

/// MCP Protocol versions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpProtocolVersion {
    /// Original protocol without streamable HTTP (2024-11-05)
    V2024_11_05,
    /// Protocol including streamable HTTP (2025-03-26)  
    V2025_03_26,
    /// Protocol with structured _meta, cursor, progressToken, and elicitation (2025-06-18)
    V2025_06_18,
}

impl McpProtocolVersion {
    /// Parse from header string
    pub fn from_str(s: &str) -> Option<Self> {
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

impl Default for McpProtocolVersion {
    fn default() -> Self {
        Self::V2025_06_18
    }
}

/// Streamable HTTP request context
#[derive(Debug, Clone)]
pub struct StreamableHttpContext {
    /// Protocol version negotiated
    pub protocol_version: McpProtocolVersion,
    /// Session ID if provided
    pub session_id: Option<String>,
    /// Whether client wants streaming responses
    pub wants_streaming: bool,
    /// Whether client accepts JSON responses
    pub accepts_json: bool,
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
            .and_then(McpProtocolVersion::from_str)
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

        let wants_streaming = accept_header.contains("text/event-stream");
        let accepts_json = accept_header.contains("application/json") || accept_header.contains("*/*");

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
            wants_streaming,
            accepts_json,
            headers: header_map,
        }
    }

    /// Check if request is compatible with streamable HTTP
    pub fn is_streamable_compatible(&self) -> bool {
        self.protocol_version.supports_streamable_http() && 
        self.wants_streaming &&
        self.session_id.is_some()
    }

    /// Validate request for MCP compliance
    pub fn validate(&self) -> std::result::Result<(), String> {
        if !self.accepts_json {
            return Err("Accept header must include application/json".to_string());
        }

        if self.wants_streaming && !self.protocol_version.supports_streamable_http() {
            return Err(format!(
                "Protocol version {} does not support streamable HTTP",
                self.protocol_version.as_str()
            ));
        }

        if self.wants_streaming && self.session_id.is_none() {
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
            self.protocol_version.as_str().parse().unwrap()
        );

        // Include session ID if present
        if let Some(session_id) = &self.session_id {
            headers.insert(
                "Mcp-Session-Id", 
                session_id.parse().unwrap()
            );
        }

        // Add capabilities header showing supported features
        let features = self.protocol_version.supported_features();
        if !features.is_empty() {
            headers.insert(
                "MCP-Capabilities",
                features.join(",").parse().unwrap()
            );
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
            Self::Error { status, message } => f.debug_struct("Error")
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
                
                let body = serde_json::to_string(&error_json)
                    .unwrap_or_else(|_| r#"{"error": {"code": 500, "message": "Internal server error"}}"#.to_string());
                
                Response::builder()
                    .status(status)
                    .body(Full::new(Bytes::from(body)))
                    .unwrap()
            }
        }
    }
}

/// Streamable HTTP transport handler
pub struct StreamableHttpHandler {
    #[allow(dead_code)] // TODO: Use config for future HTTP handler configuration
    config: Arc<ServerConfig>,
}

impl StreamableHttpHandler {
    pub fn new(config: Arc<ServerConfig>) -> Self {
        Self { config }
    }

    /// Handle incoming HTTP request with streamable HTTP support
    pub async fn handle_request<T>(&self, req: Request<T>) -> Response<Full<Bytes>>
    where
        T: Body + Send + 'static,
        T::Data: Send,
        T::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        // Parse streamable HTTP context from request
        let context = StreamableHttpContext::from_request(&req);
        
        info!(
            "Streamable HTTP request: method={}, protocol={}, session={:?}, streaming={}",
            req.method(),
            context.protocol_version.as_str(),
            context.session_id,
            context.wants_streaming
        );

        // Validate request
        if let Err(error) = context.validate() {
            warn!("Invalid streamable HTTP request: {}", error);
            return StreamableResponse::Error {
                status: StatusCode::BAD_REQUEST,
                message: error,
            }.into_response(&context);
        }

        // Route based on method and streaming capability
        match (req.method(), context.is_streamable_compatible()) {
            (&Method::GET, true) => self.handle_streaming_get(req, context).await,
            (&Method::POST, true) => self.handle_streaming_post(req, context).await,
            (&Method::POST, false) => self.handle_json_post(req, context).await,
            (&Method::DELETE, _) => self.handle_session_delete(req, context).await,
            _ => {
                StreamableResponse::Error {
                    status: StatusCode::METHOD_NOT_ALLOWED,
                    message: "Method not allowed for this endpoint".to_string(),
                }.into_response(&context)
            }
        }
    }

    /// Handle GET request for streaming connection
    async fn handle_streaming_get<T>(&self, _req: Request<T>, context: StreamableHttpContext) -> Response<Full<Bytes>>
    where
        T: Body + Send + 'static,
    {
        info!("Opening streaming connection for session: {:?}", context.session_id);
        
        // TODO: Implement actual streaming connection for Streamable HTTP
        // This would typically:
        // 1. Validate session exists and is authorized
        // 2. Create bi-directional stream with chunked transfer encoding
        // 3. Return streaming response supporting progressive message delivery
        // 4. Handle connection lifecycle and graceful termination
        
        StreamableResponse::Json(serde_json::json!({
            "status": "streaming_connection_opened",
            "session_id": context.session_id,
            "protocol_version": context.protocol_version.as_str(),
            "note": "Streaming implementation pending"
        })).into_response(&context)
    }

    /// Handle POST request with streaming response
    async fn handle_streaming_post<T>(&self, _req: Request<T>, context: StreamableHttpContext) -> Response<Full<Bytes>>
    where
        T: Body + Send + 'static,
    {
        info!("Handling streaming POST for session: {:?}", context.session_id);
        
        // TODO: Implement streaming POST handling for Streamable HTTP
        // This would typically:
        // 1. Parse JSON-RPC request(s) from chunked request body
        // 2. Process via dispatcher with session context
        // 3. Stream responses back with progressive message delivery
        // 4. Support multiple concurrent requests in the same session
        
        StreamableResponse::Json(serde_json::json!({
            "status": "streaming_post_accepted",
            "session_id": context.session_id,
            "protocol_version": context.protocol_version.as_str(),
            "note": "Streaming POST implementation pending"
        })).into_response(&context)
    }

    /// Handle POST request with JSON response
    async fn handle_json_post<T>(&self, _req: Request<T>, context: StreamableHttpContext) -> Response<Full<Bytes>>
    where
        T: Body + Send + 'static,
    {
        info!("Handling JSON POST (non-streaming)");
        
        // TODO: Implement standard JSON-RPC handling for legacy compatibility
        // This would typically:
        // 1. Parse JSON-RPC request(s) from request body
        // 2. Process via dispatcher (no session context for legacy clients)
        // 3. Return single JSON response (no streaming)
        // 4. Maintain backwards compatibility with MCP 2024-11-05 clients
        
        StreamableResponse::Json(serde_json::json!({
            "status": "json_post_handled",
            "protocol_version": context.protocol_version.as_str(),
            "streaming": false,
            "note": "JSON POST implementation pending"
        })).into_response(&context)
    }

    /// Handle DELETE request for session cleanup
    async fn handle_session_delete<T>(&self, _req: Request<T>, context: StreamableHttpContext) -> Response<Full<Bytes>>
    where
        T: Body + Send + 'static,
    {
        if let Some(session_id) = &context.session_id {
            info!("Deleting session: {}", session_id);
            
            // TODO: Implement session cleanup for Streamable HTTP
            // Should clean up session state, close any active streams,
            // and release resources associated with the session ID
            
            StreamableResponse::Json(serde_json::json!({
                "status": "session_deleted",
                "session_id": session_id,
                "note": "Session cleanup implementation pending"
            })).into_response(&context)
        } else {
            StreamableResponse::Error {
                status: StatusCode::BAD_REQUEST,
                message: "Mcp-Session-Id header required for session deletion".to_string(),
            }.into_response(&context)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_version_parsing() {
        assert_eq!(McpProtocolVersion::from_str("2024-11-05"), Some(McpProtocolVersion::V2024_11_05));
        assert_eq!(McpProtocolVersion::from_str("2025-03-26"), Some(McpProtocolVersion::V2025_03_26));
        assert_eq!(McpProtocolVersion::from_str("2025-06-18"), Some(McpProtocolVersion::V2025_06_18));
        assert_eq!(McpProtocolVersion::from_str("invalid"), None);
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
            wants_streaming: true,
            accepts_json: true,
            headers: HashMap::new(),
        };

        assert!(context.validate().is_ok());

        // Test invalid cases
        context.accepts_json = false;
        assert!(context.validate().is_err());

        context.accepts_json = true;
        context.protocol_version = McpProtocolVersion::V2024_11_05;
        context.wants_streaming = true;
        assert!(context.validate().is_err());

        context.protocol_version = McpProtocolVersion::V2025_06_18;
        context.session_id = None;
        assert!(context.validate().is_err());
    }
}