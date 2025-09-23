//! # HTTP MCP Server
//!
//! This crate provides HTTP transport for Model Context Protocol (MCP) servers.
//! It supports both modern Streamable HTTP and legacy HTTP+SSE transports
//! for maximum compatibility with all MCP clients.
//!
//! ## Supported Transports
//! - **Streamable HTTP**: Recommended for production deployments
//! - **HTTP+SSE**: Legacy transport for backwards compatibility
//!
//! ## Features
//! - Automatic protocol version detection and routing
//! - CORS support for browser-based clients
//! - Session management with cryptographically secure IDs
//! - Graceful error handling and JSON-RPC 2.0 compliance

pub mod cors;
pub mod handler;
pub mod json_rpc_responses;
pub mod mcp_session;
pub mod notification_bridge;
pub mod protocol;
pub mod server;
pub mod session_handler;
pub mod sse;
pub mod stream_manager;
pub mod streamable_http;

#[cfg(test)]
mod tests;

// Re-export main types
pub use cors::CorsLayer;
// McpHttpHandler removed in 0.2.0 - use SessionMcpHandler instead
pub use notification_bridge::{
    BroadcastError, NotificationBroadcaster, SharedNotificationBroadcaster,
    StreamManagerNotificationBroadcaster,
};
pub use protocol::{
    McpProtocolVersion, extract_last_event_id, extract_protocol_version, extract_session_id,
};
/// HTTP MCP server implementation with builder pattern and configuration
pub use server::{HttpMcpServer, HttpMcpServerBuilder, ServerConfig, ServerStats};
/// Session-aware request handler with SSE streaming capabilities
pub use session_handler::{SessionMcpHandler, SessionSseStream};
/// SSE stream management with connection tracking and event delivery
pub use stream_manager::{StreamConfig, StreamError, StreamManager, StreamStats};
pub use streamable_http::{StreamableHttpContext, StreamableHttpHandler};

// Re-export foundational types
/// JSON-RPC 2.0 dispatcher and handler trait for protocol operations
pub use turul_mcp_json_rpc_server::{JsonRpcDispatcher, JsonRpcHandler};
/// Core MCP protocol types and error handling
pub use turul_mcp_protocol::*;

/// Result type for HTTP MCP operations
pub type Result<T> = std::result::Result<T, HttpMcpError>;

/// HTTP MCP specific errors
#[derive(Debug, thiserror::Error)]
pub enum HttpMcpError {
    #[error("HTTP error: {0}")]
    Http(#[from] hyper::Error),

    #[error("JSON-RPC error: {0}")]
    JsonRpc(#[from] turul_mcp_json_rpc_server::JsonRpcError),

    #[error("MCP protocol error: {0}")]
    Mcp(#[from] turul_mcp_protocol::McpError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}
