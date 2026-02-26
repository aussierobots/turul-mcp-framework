//! Prelude module for common MCP server imports
//!
//! This module provides a convenient way to import the most commonly used
//! types and traits for building MCP servers.
//!
//! # Usage
//!
//! ```rust,no_run
//! use turul_mcp_server::prelude::*;
//!
//! // Now you have access to all common server types plus protocol types
//! ```

// Re-export all protocol prelude items (spec-pure types only)
pub use turul_mcp_protocol::prelude::*;

// Re-export all builders prelude items (includes framework traits)
pub use turul_mcp_builders::prelude::*;

// Server core types
pub use crate::{McpResult, McpServer, McpServerBuilder, SessionContext};

// HTTP server config (when available)
#[cfg(feature = "http")]
pub use crate::http::ServerConfig;

// Server trait interfaces
pub use crate::{McpPrompt, McpResource, McpTool};

// Middleware types (when HTTP feature is enabled)
#[cfg(feature = "http")]
pub use turul_http_mcp_server::middleware::{
    DispatcherResult, McpMiddleware, MiddlewareError, MiddlewareStack, RequestContext,
    SessionInjection, StorageBackedSessionView,
};

// Essential async trait for implementations
pub use async_trait::async_trait;

// Common serde types for serialization
pub use serde::{Deserialize, Serialize};

// Additional commonly used types
pub use std::sync::Arc;
pub use tokio;
