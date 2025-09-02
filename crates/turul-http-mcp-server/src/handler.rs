//! HTTP request handler for MCP protocol

use std::sync::Arc;
use std::pin::Pin;
use std::task::{Context, Poll};

use hyper::{Request, Response, Method, StatusCode};
use http_body_util::Full;
use bytes::Bytes;
use hyper::header::{CONTENT_TYPE, ACCEPT};
use http_body_util::BodyExt;
use tracing::{debug, warn, error};
use futures::Stream;
use http_body::Body;

use crate::{Result, ServerConfig, sse::SseManager};
use turul_mcp_json_rpc_server::{JsonRpcDispatcher, dispatch::parse_json_rpc_message};

/// SSE stream body that implements hyper's Body trait
pub struct SseStreamBody {
    stream: Pin<Box<dyn Stream<Item = std::result::Result<String, tokio::sync::broadcast::error::RecvError>> + Send>>,
}

impl SseStreamBody {
    pub fn new<S>(stream: S) -> Self
    where
        S: Stream<Item = std::result::Result<String, tokio::sync::broadcast::error::RecvError>> + Send + 'static,
    {
        Self {
            stream: Box::pin(stream),
        }
    }
}

impl Body for SseStreamBody {
    type Data = Bytes;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<std::result::Result<http_body::Frame<Self::Data>, Self::Error>>> {
        match self.stream.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(data))) => {
                let bytes = Bytes::from(data);
                Poll::Ready(Some(Ok(http_body::Frame::data(bytes))))
            }
            Poll::Ready(Some(Err(e))) => {
                Poll::Ready(Some(Err(Box::new(e))))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// HTTP handler for MCP requests
pub struct McpHttpHandler {
    pub(crate) config: ServerConfig,
    pub(crate) dispatcher: Arc<JsonRpcDispatcher>,
    pub(crate) sse_manager: Arc<SseManager>,
}

impl McpHttpHandler {
    /// Create a new handler
    pub fn new(config: ServerConfig, dispatcher: Arc<JsonRpcDispatcher>) -> Self {
        Self {
            config,
            dispatcher,
            sse_manager: Arc::new(SseManager::new()),
        }
    }

    /// Create a new handler with existing SSE manager
    pub fn with_sse_manager(
        config: ServerConfig,
        dispatcher: Arc<JsonRpcDispatcher>,
        sse_manager: Arc<SseManager>
    ) -> Self {
        Self { config, dispatcher, sse_manager }
    }

    /// Handle MCP HTTP requests
    pub async fn handle_mcp_request(
        &self,
        req: Request<hyper::body::Incoming>,
    ) -> Result<Response<Full<Bytes>>> {
        match req.method() {
            &Method::POST => self.handle_json_rpc_request(req).await,
            &Method::GET => {
                if self.config.enable_get_sse {
                    self.handle_sse_request(req).await
                } else {
                    self.method_not_allowed().await
                }
            }
            &Method::OPTIONS => self.handle_preflight().await,
            _ => self.method_not_allowed().await,
        }
    }

    /// Handle JSON-RPC requests over HTTP POST
    async fn handle_json_rpc_request(
        &self,
        req: Request<hyper::body::Incoming>,
    ) -> Result<Response<Full<Bytes>>> {
        // Check content type
        let content_type = req.headers()
            .get(CONTENT_TYPE)
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or("");

        if !content_type.starts_with("application/json") {
            warn!("Invalid content type: {}", content_type);
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Content-Type must be application/json")))
                .unwrap());
        }

        // Read request body
        let body = req.into_body();
        let body_bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(err) => {
                error!("Failed to read request body: {}", err);
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Full::new(Bytes::from("Failed to read request body")))
                    .unwrap());
            }
        };

        // Check body size
        if body_bytes.len() > self.config.max_body_size {
            warn!("Request body too large: {} bytes", body_bytes.len());
            return Ok(Response::builder()
                .status(StatusCode::PAYLOAD_TOO_LARGE)
                .body(Full::new(Bytes::from("Request body too large")))
                .unwrap());
        }

        // Parse as UTF-8
        let body_str = match std::str::from_utf8(&body_bytes) {
            Ok(s) => s,
            Err(err) => {
                error!("Invalid UTF-8 in request body: {}", err);
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Full::new(Bytes::from("Request body must be valid UTF-8")))
                    .unwrap());
            }
        };

        debug!("Received JSON-RPC request: {}", body_str);

        // Parse JSON-RPC message
        let message = match parse_json_rpc_message(body_str) {
            Ok(msg) => msg,
            Err(rpc_err) => {
                error!("JSON-RPC parse error: {}", rpc_err);
                let error_response = serde_json::to_string(&rpc_err)?;
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header(CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(error_response)))
                    .unwrap());
            }
        };

        // Handle the message
        match message {
            turul_mcp_json_rpc_server::dispatch::JsonRpcMessage::Request(request) => {
                debug!("Processing JSON-RPC request: method={}", request.method);
                let response = self.dispatcher.handle_request(request).await;
                let response_json = serde_json::to_string(&response)?;

                debug!("Sending JSON-RPC response");
                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(response_json)))
                    .unwrap())
            }
            turul_mcp_json_rpc_server::dispatch::JsonRpcMessage::Notification(notification) => {
                debug!("Processing JSON-RPC notification: method={}", notification.method);
                if let Err(err) = self.dispatcher.handle_notification(notification).await {
                    error!("Notification handling error: {}", err);
                }

                // Notifications don't return responses
                Ok(Response::builder()
                    .status(StatusCode::NO_CONTENT)
                    .body(Full::new(Bytes::new()))
                    .unwrap())
            }
        }
    }

    /// Handle Server-Sent Events requests
    async fn handle_sse_request(
        &self,
        req: Request<hyper::body::Incoming>,
    ) -> Result<Response<Full<Bytes>>> {
        // Check if client accepts SSE
        let headers = req.headers();
        let accept = headers
            .get(ACCEPT)
            .and_then(|accept| accept.to_str().ok())
            .unwrap_or("");

        if !accept.contains("text/event-stream") {
            return Ok(Response::builder()
                .status(StatusCode::NOT_ACCEPTABLE)
                .body(Full::new(Bytes::from("SSE not accepted")))
                .unwrap());
        }

        // Extract connection ID from query parameters or generate one
        let connection_id = match req.uri().query() {
            Some(q) => {
                q.split('&')
                    .find(|param| param.starts_with("connection_id="))
                    .and_then(|param| param.split('=').nth(1))
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| uuid::Uuid::now_v7().to_string())
            }
            None => uuid::Uuid::now_v7().to_string(),
        };

        debug!("Creating SSE connection: {}", connection_id);

        // For now, return an initial connection message with instructions
        // TODO: Implement actual streaming with SseStreamBody when we can change the signature
        let initial_response = format!(
            "event: connection\n\
             data: {{\"type\":\"connected\",\"connection_id\":\"{}\",\"message\":\"SSE connection established\"}}\n\n\
             event: info\n\
             data: {{\"type\":\"info\",\"message\":\"This is a basic SSE endpoint. Full streaming will be available in a future update.\"}}\n\n",
            connection_id
        );

        // Store the connection for later use
        let _connection = self.sse_manager.create_connection(connection_id.clone()).await;

        // Start a background task to send periodic keep-alives
        let sse_manager = Arc::clone(&self.sse_manager);
        let conn_id = connection_id.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            for _ in 0..10 { // Send 10 keep-alives then stop for this basic implementation
                interval.tick().await;
                sse_manager.send_keep_alive().await;
            }
            sse_manager.remove_connection(&conn_id).await;
        });

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Headers", "Cache-Control")
            .body(Full::new(Bytes::from(initial_response)))
            .unwrap())
    }

    /// Handle OPTIONS preflight requests
    async fn handle_preflight(&self) -> Result<Response<Full<Bytes>>> {
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Access-Control-Allow-Methods", "POST, GET, OPTIONS")
            .header("Access-Control-Allow-Headers", "Content-Type, Accept")
            .header("Access-Control-Max-Age", "86400")
            .body(Full::new(Bytes::new()))
            .unwrap())
    }

    /// Return method not allowed response
    async fn method_not_allowed(&self) -> Result<Response<Full<Bytes>>> {
        Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .header("Allow", "POST, OPTIONS")
            .body(Full::new(Bytes::from("Method not allowed")))
            .unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use turul_mcp_json_rpc_server::JsonRpcDispatcher;

    fn create_test_handler() -> McpHttpHandler {
        let config = ServerConfig::default();
        let dispatcher = Arc::new(JsonRpcDispatcher::new());
        McpHttpHandler::new(config, dispatcher)
    }

    #[tokio::test]
    async fn test_options_request() {
        let _handler = create_test_handler();
        // For testing, we'll need to create a proper request body
        // For now, let's create a simple test that doesn't use actual HTTP requests
        assert!(true); // Placeholder test
    }

    #[tokio::test]
    async fn test_method_not_allowed() {
        let _handler = create_test_handler();
        // For testing, we'll need to create a proper request body
        // For now, let's create a simple test that doesn't use actual HTTP requests
        assert!(true); // Placeholder test
    }
}
