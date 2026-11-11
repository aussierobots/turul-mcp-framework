//! HTTP transport for [Model Context Protocol](https://modelcontextprotocol.io) servers:
//! Streamable HTTP routing, CORS, Origin checks and SSE streaming on top of hyper.
//!
//! Most users want [`turul-mcp-server`](https://docs.rs/turul-mcp-server), which
//! builds on this crate and adds tools, resources, prompts and macros. Use this
//! crate directly only to embed the HTTP transport around your own JSON-RPC
//! handlers.
//!
//! # Example
//!
//! ```toml
//! [dependencies]
//! turul-http-mcp-server = "0.4"
//! turul-mcp-protocol = "0.4"
//! turul-rpc = "0.2"
//! async-trait = "0.1"
//! serde_json = "1"
//! tokio = { version = "1", features = ["full"] }
//! ```
//!
//! ```rust,no_run
//! use turul_http_mcp_server::{HttpMcpServerBuilder, JsonRpcHandler, McpError};
//! use turul_mcp_protocol::tools::{ListToolsResult, Tool, ToolSchema};
//!
//! struct ToolList;
//!
//! #[async_trait::async_trait]
//! impl JsonRpcHandler for ToolList {
//!     type Error = McpError;
//!
//!     async fn handle(
//!         &self,
//!         _method: &str,
//!         _params: Option<turul_rpc::RequestParams>,
//!         _session: Option<turul_rpc::SessionContext>,
//!     ) -> Result<serde_json::Value, McpError> {
//!         let tools = ListToolsResult::new(vec![Tool::new("ping", ToolSchema::object())]);
//!         Ok(serde_json::to_value(tools)?)
//!     }
//! }
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let server = HttpMcpServerBuilder::new()
//!     .bind_address("127.0.0.1:8000".parse()?)
//!     .register_handler(vec!["tools/list".to_string()], ToolList)
//!     .build();
//! server.run().await?;
//! # Ok(())
//! # }
//! ```
//!
//! The endpoint is `http://127.0.0.1:8080/mcp`; change it with
//! [`HttpMcpServerBuilder::mcp_path`].
//!
//! # Features
//!
//! | Feature | Default | Enables |
//! |---|---|---|
//! | `sse` | yes | Server-sent event streams |
//! | `protocol-2026-07-28` | yes | MCP 2026-07-28 (stateless core) |
//! | `protocol-2025-11-25` | no | MCP 2025-11-25 sessions; use `--no-default-features --features sse,protocol-2025-11-25` |
//!
//! # Where next
//!
//! - [`HttpMcpServerBuilder`] for every transport option
//! - [`StreamableHttpHandler`] to mount the transport in your own hyper service
//! - [`turul-mcp-server`](https://docs.rs/turul-mcp-server) for the full server
//! - [Examples](https://github.com/aussierobots/turul-mcp-framework/tree/main/examples)

pub mod cors;
pub mod handler;
pub mod json_rpc_responses;
pub mod mcp_session;
pub mod middleware;
pub mod notification_bridge;
pub mod origin;
pub mod prelude;
pub mod protocol;
pub mod routes;
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
pub use origin::OriginPolicy;
pub use protocol::{
    McpProtocolVersion, extract_last_event_id, extract_protocol_version, extract_session_id,
};
pub use routes::{RouteBody, RouteHandler, RouteRegistry, RouteValidationError};
/// HTTP MCP server implementation with builder pattern and configuration
pub use server::{HttpMcpServer, HttpMcpServerBuilder, ServerConfig, ServerStats};
/// Session-aware request handler with SSE streaming capabilities
pub use session_handler::{SessionMcpHandler, SessionSseStream};
/// SSE stream management with connection tracking and event delivery
pub use stream_manager::{StreamConfig, StreamError, StreamManager, StreamStats};
pub use streamable_http::{StreamableHttpContext, StreamableHttpHandler};

/// Awaitable notifier for tool change events (restart/redeploy fingerprint mismatch).
/// Implemented by the server layer (backed by SessionManager → dispatcher).
/// Used by HTTP handlers to persist `notifications/tools/list_changed` through
/// the session architecture, not directly via StreamManager.
#[async_trait::async_trait]
pub trait ToolChangeNotifier: Send + Sync {
    async fn notify_tools_changed(&self, session_id: &str) -> std::result::Result<(), String>;
}

// Re-export foundational types
/// Core MCP protocol types and error handling
pub use turul_mcp_protocol::*;
/// JSON-RPC 2.0 dispatcher and handler trait for protocol operations
pub use turul_rpc::{JsonRpcDispatcher, JsonRpcHandler};

/// Result type for HTTP MCP operations
pub type Result<T> = std::result::Result<T, HttpMcpError>;

/// HTTP MCP specific errors
#[derive(Debug, thiserror::Error)]
pub enum HttpMcpError {
    #[error("HTTP error: {0}")]
    Http(#[from] hyper::Error),

    #[error("JSON-RPC error: {0}")]
    JsonRpc(#[from] turul_rpc::JsonRpcError),

    #[error("MCP protocol error: {0}")]
    Mcp(#[from] turul_mcp_protocol::McpError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}
