//! JSON-RPC 2.0 over HTTP handler for MCP requests
//!
//! This handler implements proper JSON-RPC 2.0 server over HTTP transport,
//! always returning JSON-RPC 2.0 conformant responses.

use std::sync::Arc;
use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};

use hyper::{Request, Response, Method, StatusCode};
use bytes::Bytes;
use hyper::header::{CONTENT_TYPE, ACCEPT};
use http_body_util::{BodyExt, Full};
use tracing::{debug, warn, error};
use futures::Stream;
use http_body::{Body, Frame};

use json_rpc_server::{
    JsonRpcDispatcher,
    dispatch::{parse_json_rpc_message, JsonRpcMessage, JsonRpcMessageResult}
};
use crate::{
    Result, ServerConfig, 
    protocol::{extract_protocol_version, extract_session_id}, 
    mcp_session,
    json_rpc_responses::*
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

/// JSON-RPC 2.0 over HTTP handler for MCP requests
pub struct SessionMcpHandler {
    pub(crate) config: ServerConfig,
    pub(crate) dispatcher: Arc<JsonRpcDispatcher>,
}

impl SessionMcpHandler {
    /// Create a new JSON-RPC 2.0 over HTTP handler
    pub fn new(
        config: ServerConfig, 
        dispatcher: Arc<JsonRpcDispatcher>,
    ) -> Self {
        Self { config, dispatcher }
    }

    /// Create session context for request handling (future implementation)
    fn create_session_context(&self, _session_id: &str) -> serde_json::Value {
        // Placeholder for future session context integration
        serde_json::json!({
            "session_id": _session_id,
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
    }

    /// Handle MCP HTTP requests - returns proper JSON-RPC 2.0 responses
    pub async fn handle_mcp_request(
        &self,
        req: Request<hyper::body::Incoming>,
    ) -> Result<Response<JsonRpcBody>> {
        match req.method() {
            &Method::POST => self.handle_json_rpc_request(req).await,
            &Method::GET => self.handle_sse_request(req).await,
            &Method::DELETE => self.handle_delete_request(req).await,
            &Method::OPTIONS => Ok(self.handle_preflight()),
            _ => Ok(self.method_not_allowed()),
        }
    }

    /// Handle JSON-RPC requests over HTTP POST - returns JSON-RPC 2.0 conformant responses
    async fn handle_json_rpc_request(
        &self,
        req: Request<hyper::body::Incoming>,
    ) -> Result<Response<JsonRpcBody>> {
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
        let message_result = match message {
            JsonRpcMessage::Request(request) => {
                debug!("Processing JSON-RPC request: method={}", request.method);
                
                // Get session ID for context if available
                let _session_context = session_id.as_ref().map(|id| self.create_session_context(id));
                
                // Dispatch the request
                let response = self.dispatcher.handle_request(request).await;
                JsonRpcMessageResult::Response(response)
            }
            JsonRpcMessage::Notification(notification) => {
                debug!("Processing JSON-RPC notification: method={}", notification.method);
                if let Err(err) = self.dispatcher.handle_notification(notification).await {
                    error!("Notification handling error: {}", err);
                }
                JsonRpcMessageResult::NoResponse
            }
        };

        // Convert message result to HTTP response (always JSON-RPC 2.0 conformant)
        match message_result {
            JsonRpcMessageResult::Response(response) => {
                debug!("Sending JSON-RPC response");
                Ok(jsonrpc_response_with_session(response, session_id.map(|s| s.to_string()))?)
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

    /// Handle Server-Sent Events requests (SSE for streaming)
    async fn handle_sse_request(
        &self,
        req: Request<hyper::body::Incoming>,
    ) -> Result<Response<JsonRpcBody>> {
        // Check if client accepts SSE
        let headers = req.headers();
        let accept = headers
            .get(ACCEPT)
            .and_then(|accept| accept.to_str().ok())
            .unwrap_or("");

        if !accept.contains("text/event-stream") {
            warn!("GET request received without SSE support - header does not contain 'text/event-stream'");
            return Ok(Response::builder()
                .status(StatusCode::NOT_ACCEPTABLE)
                .body(Full::new(Bytes::from("SSE not accepted - missing 'text/event-stream' in Accept header")))
                .unwrap());
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
                return Ok(Response::builder()
                    .status(StatusCode::PRECONDITION_FAILED)
                    .body(Full::new(Bytes::from("Missing Mcp-Session-Id header")))
                    .unwrap());
            }
        };

        // Check if session exists and touch it
        if !mcp_session::session_exists(&session_id).await {
            error!("Session not found for Session ID: {}", session_id);
            return Ok(Response::builder()
                .status(StatusCode::PRECONDITION_FAILED)
                .body(Full::new(Bytes::from(format!("Session not found for Session ID: {}", session_id))))
                .unwrap());
        }

        // Get the session's event receiver
        let receiver = match mcp_session::get_receiver(&session_id).await {
            Some(receiver) => receiver,
            None => {
                error!("Failed to get event receiver for session: {}", session_id);
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Full::new(Bytes::from("Session access error")))
                    .unwrap());
            }
        };

        debug!("Creating SSE stream for session: {}", session_id);

        // SSE stream implementation would go here
        let _receiver = receiver; // Use receiver to avoid unused warning

        // For now, return a simple message indicating SSE is not yet fully implemented
        // TODO: Implement proper SSE streaming with SessionSseStream
        warn!("SSE streaming not yet fully implemented in new architecture");
        Ok(Response::builder()
            .status(StatusCode::NOT_IMPLEMENTED)
            .body(Full::new(Bytes::from("SSE streaming not yet implemented")))
            .unwrap())
    }

    /// Handle DELETE requests for session cleanup
    async fn handle_delete_request(
        &self,
        req: Request<hyper::body::Incoming>,
    ) -> Result<Response<JsonRpcBody>> {
        let session_id = extract_session_id(req.headers());
        
        debug!("DELETE request - Session: {:?}", session_id);

        if let Some(session_id) = session_id {
            if mcp_session::remove_session(&session_id).await {
                debug!("Session {} removed via DELETE", session_id);
                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .body(Full::new(Bytes::from("Session removed")))
                    .unwrap())
            } else {
                Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Full::new(Bytes::from("Session not found")))
                    .unwrap())
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
}


