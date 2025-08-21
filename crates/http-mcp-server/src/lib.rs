//! # HTTP MCP Server
//!
//! This crate provides HTTP transport for Model Context Protocol (MCP) servers.
//! It supports both modern Streamable HTTP (2025-03-26+) and legacy HTTP+SSE transports
//! for maximum compatibility with all MCP clients.
//!
//! ## Supported Transports
//! - **Streamable HTTP (2025-03-26+)**: Recommended for production deployments
//! - **HTTP+SSE (2024-11-05)**: Legacy transport for backwards compatibility
//!
//! ## Features
//! - Automatic protocol version detection and routing
//! - CORS support for browser-based clients
//! - Session management with cryptographically secure IDs
//! - Graceful error handling and JSON-RPC 2.0 compliance

pub mod server;
pub mod handler;
pub mod cors;
pub mod sse;
pub mod streamable_http;

#[cfg(test)]
mod tests;

// Re-export main types
pub use server::{HttpMcpServer, HttpMcpServerBuilder, ServerConfig};
pub use handler::McpHttpHandler;
pub use cors::CorsLayer;
pub use streamable_http::{StreamableHttpHandler, StreamableHttpContext, McpProtocolVersion};

// Re-export foundational types
pub use json_rpc_server::{JsonRpcHandler, JsonRpcDispatcher};
pub use mcp_protocol::*;

/// Result type for HTTP MCP operations
pub type Result<T> = std::result::Result<T, HttpMcpError>;

/// HTTP MCP specific errors
#[derive(Debug, thiserror::Error)]
pub enum HttpMcpError {
    #[error("HTTP error: {0}")]
    Http(#[from] hyper::Error),
    
    #[error("JSON-RPC error: {0}")]
    JsonRpc(#[from] json_rpc_server::JsonRpcError),
    
    #[error("MCP protocol error: {0}")]
    Mcp(#[from] mcp_protocol::McpError),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}
