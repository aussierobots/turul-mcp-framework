//! # HTTP MCP Server Prelude
//!
//! This module provides convenient re-exports of the most commonly used types
//! from the HTTP MCP server library.
//!
//! ```rust
//! use turul_http_mcp_server::prelude::*;
//! ```

// Core server types
pub use crate::server::{HttpMcpServer, HttpMcpServerBuilder, ServerConfig, ServerStats};
pub use crate::session_handler::{SessionMcpHandler, SessionSseStream};
pub use crate::stream_manager::{StreamConfig, StreamError, StreamManager, StreamStats};
pub use crate::cors::CorsLayer;

// Protocol and notification types
pub use crate::notification_bridge::{
    BroadcastError, NotificationBroadcaster, SharedNotificationBroadcaster,
    StreamManagerNotificationBroadcaster,
};
pub use crate::protocol::{
    McpProtocolVersion, extract_last_event_id, extract_protocol_version, extract_session_id,
};

// Re-export foundational types
pub use crate::{JsonRpcDispatcher, JsonRpcHandler};
pub use turul_mcp_protocol::prelude::*;

// Error types
pub use crate::{HttpMcpError, Result};