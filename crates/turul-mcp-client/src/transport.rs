//! Transport layer for MCP client

use async_trait::async_trait;
use serde_json::Value;
use url::Url;

use crate::error::{McpClientResult, TransportError};

pub mod http;
pub mod sse;

#[cfg(feature = "websocket")]
pub mod websocket;

#[cfg(feature = "stdio")]
pub mod stdio;

// Re-export transport implementations
pub use http::HttpTransport;
pub use sse::SseTransport;

#[cfg(feature = "websocket")]
pub use websocket::WebSocketTransport;

#[cfg(feature = "stdio")]
pub use stdio::StdioTransport;

/// Transport type enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum TransportType {
    /// HTTP transport (Streamable HTTP 2025-03-26+)
    Http,
    /// Server-Sent Events transport (HTTP+SSE 2024-11-05)
    Sse,
    /// WebSocket transport
    WebSocket,
    /// Standard I/O transport (for local processes)
    Stdio,
}

impl std::fmt::Display for TransportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportType::Http => write!(f, "HTTP"),
            TransportType::Sse => write!(f, "SSE"),
            TransportType::WebSocket => write!(f, "WebSocket"),
            TransportType::Stdio => write!(f, "Stdio"),
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

/// Transport trait defining the interface for all transport implementations
#[async_trait]
pub trait Transport: Send + Sync {
    /// Get transport type
    fn transport_type(&self) -> TransportType;
    
    /// Get transport capabilities
    fn capabilities(&self) -> TransportCapabilities;
    
    /// Connect to the server
    async fn connect(&mut self) -> McpClientResult<()>;
    
    /// Disconnect from the server
    async fn disconnect(&mut self) -> McpClientResult<()>;
    
    /// Check if connected
    fn is_connected(&self) -> bool;
    
    /// Send a request and wait for response
    async fn send_request(&mut self, request: Value) -> McpClientResult<Value>;
    
    /// Send a notification (no response expected)
    async fn send_notification(&mut self, notification: Value) -> McpClientResult<()>;
    
    /// Start listening for server events (if supported)
    async fn start_event_listener(&mut self) -> McpClientResult<EventReceiver>;
    
    /// Get connection information
    fn connection_info(&self) -> ConnectionInfo;
    
    /// Perform health check
    async fn health_check(&mut self) -> McpClientResult<bool> {
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
    /// Server sent a notification
    Notification(Value),
    /// Server sent a request (requiring response)
    Request(Value),
    /// Connection was lost
    ConnectionLost,
    /// Transport error occurred
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
        "ws" | "wss" => Ok(TransportType::WebSocket),
        "stdio" | "file" => Ok(TransportType::Stdio),
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
            #[cfg(feature = "websocket")]
            TransportType::WebSocket => Ok(Box::new(WebSocketTransport::new(url)?)),
            #[cfg(feature = "stdio")]
            TransportType::Stdio => Ok(Box::new(StdioTransport::new(url)?)),
            #[cfg(not(feature = "websocket"))]
            TransportType::WebSocket => Err(TransportError::Unsupported(
                "WebSocket support not enabled".to_string()
            ).into()),
            #[cfg(not(feature = "stdio"))]
            TransportType::Stdio => Err(TransportError::Unsupported(
                "Stdio support not enabled".to_string()
            ).into()),
        }
    }
    
    /// Create a specific transport type
    pub fn create(transport_type: TransportType, endpoint: &str) -> McpClientResult<BoxedTransport> {
        match transport_type {
            TransportType::Http => Ok(Box::new(HttpTransport::new(endpoint)?)),
            TransportType::Sse => Ok(Box::new(SseTransport::new(endpoint)?)),
            #[cfg(feature = "websocket")]
            TransportType::WebSocket => Ok(Box::new(WebSocketTransport::new(endpoint)?)),
            #[cfg(feature = "stdio")]
            TransportType::Stdio => Ok(Box::new(StdioTransport::new(endpoint)?)),
            #[cfg(not(feature = "websocket"))]
            TransportType::WebSocket => Err(TransportError::Unsupported(
                "WebSocket support not enabled".to_string()
            ).into()),
            #[cfg(not(feature = "stdio"))]
            TransportType::Stdio => Err(TransportError::Unsupported(
                "Stdio support not enabled".to_string()
            ).into()),
        }
    }
    
    /// List available transport types
    pub fn available_transports() -> Vec<TransportType> {
        let transports = vec![TransportType::Http, TransportType::Sse];
        
        #[cfg(feature = "websocket")]
        transports.push(TransportType::WebSocket);
        
        #[cfg(feature = "stdio")]
        transports.push(TransportType::Stdio);
        
        transports
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
        
        assert_eq!(
            detect_transport_type("ws://localhost:8080/mcp").unwrap(),
            TransportType::WebSocket
        );
        
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