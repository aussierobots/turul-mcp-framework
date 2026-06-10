//! Transport layer for MCP client

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use url::Url;

use crate::error::{McpClientResult, TransportError};

pub mod http;
pub mod sse;

// Stdio transport is planned for future implementation

// #[cfg(feature = "stdio")]
// pub mod stdio;

// Re-export transport implementations
pub use http::HttpTransport;
pub use sse::SseTransport;

// Re-exports for future transport implementations
// #[cfg(feature = "stdio")]
// pub use stdio::StdioTransport;

/// Transport type enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum TransportType {
    /// HTTP transport (Streamable HTTP)
    Http,
    /// Server-Sent Events transport (HTTP+SSE)
    Sse,
    // Future transport types:
    // Stdio,
}

impl std::fmt::Display for TransportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportType::Http => write!(f, "HTTP"),
            TransportType::Sse => write!(f, "SSE"),
        }
    }
}

/// Transport capabilities
#[derive(Debug, Clone)]
pub struct TransportCapabilities {
    /// Whether the transport supports streaming responses
    pub streaming: bool,
    /// Whether the transport supports bidirectional communication
    pub bidirectional: bool,
    /// Whether the transport supports server-initiated events
    pub server_events: bool,
    /// Maximum message size (if applicable)
    pub max_message_size: Option<usize>,
    /// Whether the transport maintains persistent connections
    pub persistent: bool,
}

/// Transport connection information
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// Transport type
    pub transport_type: TransportType,
    /// Connection endpoint
    pub endpoint: String,
    /// Connection state
    pub connected: bool,
    /// Transport capabilities
    pub capabilities: TransportCapabilities,
    /// Additional metadata
    pub metadata: Value,
}

/// Transport response containing both body and headers
#[derive(Debug, Clone)]
pub struct TransportResponse {
    /// Response body (JSON)
    pub body: Value,
    /// Response headers
    pub headers: HashMap<String, String>,
}

impl TransportResponse {
    /// Create a new transport response
    pub fn new(body: Value, headers: HashMap<String, String>) -> Self {
        Self { body, headers }
    }

    /// Create a simple response with just body (no headers)
    pub fn body_only(body: Value) -> Self {
        Self {
            body,
            headers: HashMap::new(),
        }
    }
}

/// Transport trait defining the interface for all transport implementations
#[async_trait]
pub trait Transport: Send + Sync {
    /// Get transport type
    fn transport_type(&self) -> TransportType;

    /// Get transport capabilities
    fn capabilities(&self) -> TransportCapabilities;

    /// Connect to the server
    async fn connect(&self) -> McpClientResult<()>;

    /// Disconnect from the server
    async fn disconnect(&self) -> McpClientResult<()>;

    /// Check if connected
    fn is_connected(&self) -> bool;

    /// Send a request and wait for response
    async fn send_request(&self, request: Value) -> McpClientResult<Value>;

    /// Send a request with additional per-request HTTP headers (SEP-2243
    /// `Mcp-Param-*` mirrors). Non-HTTP transports MAY ignore the annotations
    /// entirely, so the default delegates to [`Self::send_request`].
    async fn send_request_with_extra_headers(
        &self,
        request: Value,
        _extra_headers: &[(String, String)],
    ) -> McpClientResult<Value> {
        self.send_request(request).await
    }

    /// Send a request and return response with headers (for initialization)
    async fn send_request_with_headers(&self, request: Value)
    -> McpClientResult<TransportResponse>;

    /// Send a notification (no response expected)
    async fn send_notification(&self, notification: Value) -> McpClientResult<()>;

    /// Send a DELETE request for session termination (MCP session management)
    async fn send_delete(&self, session_id: &str) -> McpClientResult<()>;

    /// Set the session ID to include in subsequent requests (MCP session management)
    fn set_session_id(&self, session_id: String);

    /// Clear the session ID (used during 404 re-initialization)
    fn clear_session_id(&self);

    /// Update (or clear) the `Authorization` header used on subsequent outbound
    /// requests, *without* tearing down the underlying connection pool.
    ///
    /// `Some("Bearer …")` overrides any previously-configured Authorization
    /// header. `None` removes the override, falling back to whatever was
    /// configured at transport construction (e.g. via
    /// `ConnectionConfig::headers`).
    ///
    /// # Why this exists
    ///
    /// Transports that cache static headers at construction (e.g. the HTTP
    /// transport's `reqwest::default_headers`) will otherwise send a stale
    /// bearer on long-running clients that outlive a single token. The
    /// canonical failure is OAuth `client_credentials` rotation: a caller mints
    /// a fresh bearer for one [`crate::McpClient`], but a previously-created
    /// client still holds the old bearer in `default_headers`, so its
    /// `disconnect()` DELETE flies under a token the AS may have already
    /// revoked — observed as `HTTP 403 Forbidden` from upstream authorizers
    /// (API Gateway, ALB OIDC, etc.).
    ///
    /// Updating the override before calling `disconnect()` lets the DELETE
    /// carry the current bearer instead.
    ///
    /// # Default implementation
    ///
    /// Transports that do not authenticate at the transport layer (stdio, plain
    /// SSE) ignore this call. Only `HttpTransport` is expected to apply it.
    async fn update_auth_header(&self, _value: Option<String>) {
        // No-op for transports without per-request HTTP auth.
    }

    /// Set the negotiated MCP spec version, sent as `MCP-Protocol-Version`.
    ///
    /// The client calls this after negotiation so the transport advertises the
    /// agreed spec (and, for the 2026-07-28 stateless core, stops sending the
    /// removed `Mcp-Session-Id`). No-op for transports that don't carry the header.
    fn set_protocol_version(&self, _version: &str) {}

    /// Start listening for server events (if supported).
    ///
    /// # Streamable HTTP listener termination (since 0.3.38)
    ///
    /// On the streamable HTTP transport, the SSE GET listener treats HTTP 4xx
    /// as **terminal**: the cached `Mcp-Session-Id` is cleared, a single
    /// [`ServerEvent::Error`] (`"SSE GET rejected with HTTP <status> — listener
    /// exiting"`) is emitted, and the spawned task exits. The returned
    /// [`EventReceiver`] is **not** closed (the transport keeps an internal
    /// sender clone), so callers must observe the terminal `Error` event itself
    /// — not channel closure — to detect the exit.
    ///
    /// Recovery is the caller's responsibility: re-run `initialize` and call
    /// `start_event_listener` again. Because the cached session header was
    /// cleared, the next initialize POST will be sent without a stale
    /// `Mcp-Session-Id`, mirroring the existing POST 404 recovery flow.
    ///
    /// HTTP 5xx and network errors remain transient and are retried with the
    /// existing static backoff.
    async fn start_event_listener(&self) -> McpClientResult<EventReceiver>;

    /// Get connection information
    fn connection_info(&self) -> ConnectionInfo;

    /// Perform health check
    async fn health_check(&self) -> McpClientResult<bool> {
        // Default implementation: try to send a ping
        let ping_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "health_check",
            "method": "ping",
            "params": {}
        });

        match self.send_request(ping_request).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Get transport statistics
    fn statistics(&self) -> TransportStatistics {
        TransportStatistics::default()
    }
}

/// Type alias for a boxed transport
pub type BoxedTransport = Box<dyn Transport>;

/// Event receiver for server-initiated events
pub type EventReceiver = tokio::sync::mpsc::UnboundedReceiver<ServerEvent>;

/// Server-initiated events
#[derive(Debug, Clone)]
pub enum ServerEvent {
    /// Server sent a notification (method only, no id)
    Notification(Value),
    /// Server sent a request requiring response (method + id)
    Request(Value),
    /// Response to a client-originated request (id only, no method).
    /// Received via SSE when server streams responses asynchronously.
    Response(Value),
    /// Connection was lost
    ConnectionLost,
    /// Transport error occurred.
    ///
    /// On the streamable HTTP transport, an `Error` whose payload contains
    /// `"listener exiting"` signals that the SSE GET listener task has
    /// terminated (HTTP 4xx response — see [`Transport::start_event_listener`]
    /// for the full contract). The cached session header has already been
    /// cleared; the caller should re-run `initialize` and restart the listener.
    /// Other `Error` payloads are non-terminal — the listener continues to
    /// retry transient failures.
    Error(String),
    /// Heartbeat/keep-alive
    Heartbeat,
}

/// Transport statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct TransportStatistics {
    /// Number of requests sent
    pub requests_sent: u64,
    /// Number of responses received
    pub responses_received: u64,
    /// Number of notifications sent
    pub notifications_sent: u64,
    /// Number of server events received
    pub events_received: u64,
    /// Number of errors encountered
    pub errors: u64,
    /// Average response time
    pub avg_response_time_ms: f64,
    /// Last error message
    pub last_error: Option<String>,
}

/// Helper function to detect transport type from URL
pub fn detect_transport_type(url_str: &str) -> McpClientResult<TransportType> {
    let url = Url::parse(url_str)
        .map_err(|e| TransportError::ConnectionFailed(format!("Invalid URL: {}", e)))?;

    match url.scheme() {
        "http" | "https" => {
            // Check for SSE hint in path or query
            if url.path().contains("/sse") || url.query().unwrap_or("").contains("transport=sse") {
                Ok(TransportType::Sse)
            } else {
                Ok(TransportType::Http)
            }
        }
        "stdio" | "file" => Err(TransportError::Unsupported(
            "Stdio transport not yet implemented".to_string(),
        )
        .into()),
        scheme => Err(TransportError::Unsupported(format!("Unknown scheme: {}", scheme)).into()),
    }
}

/// Transport factory for creating transport instances
pub struct TransportFactory;

impl TransportFactory {
    /// Create a transport from URL string
    pub fn from_url(url: &str) -> McpClientResult<BoxedTransport> {
        let transport_type = detect_transport_type(url)?;

        match transport_type {
            TransportType::Http => Ok(Box::new(HttpTransport::new(url)?)),
            TransportType::Sse => Ok(Box::new(SseTransport::new(url)?)),
        }
    }

    /// Create a specific transport type
    pub fn create(
        transport_type: TransportType,
        endpoint: &str,
    ) -> McpClientResult<BoxedTransport> {
        match transport_type {
            TransportType::Http => Ok(Box::new(HttpTransport::new(endpoint)?)),
            TransportType::Sse => Ok(Box::new(SseTransport::new(endpoint)?)),
        }
    }

    /// List available transport types
    pub fn available_transports() -> Vec<TransportType> {
        vec![TransportType::Http, TransportType::Sse]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_type_detection() {
        assert_eq!(
            detect_transport_type("http://localhost:8080/mcp").unwrap(),
            TransportType::Http
        );

        assert_eq!(
            detect_transport_type("http://localhost:8080/mcp/sse").unwrap(),
            TransportType::Sse
        );

        // Non-HTTP schemes are rejected
        assert!(detect_transport_type("ftp://localhost:8080/mcp").is_err());

        assert!(detect_transport_type("invalid://localhost").is_err());
    }

    #[test]
    fn test_transport_factory() {
        let transport = TransportFactory::from_url("http://localhost:8080/mcp").unwrap();
        assert_eq!(transport.transport_type(), TransportType::Http);

        let transports = TransportFactory::available_transports();
        assert!(transports.contains(&TransportType::Http));
        assert!(transports.contains(&TransportType::Sse));
    }
}
