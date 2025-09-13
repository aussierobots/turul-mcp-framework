//! Prelude module for common MCP server imports
//!
//! This module provides a convenient way to import the most commonly used
//! types and traits for building MCP servers.
//!
//! # Usage
//!
//! ```rust
//! use turul_mcp_server::prelude::*;
//! 
//! // Now you have access to all common server types plus protocol types
//! ```

// Re-export all protocol prelude items
pub use turul_mcp_protocol::prelude::*;

// Server core types
pub use crate::{
    McpServer, McpServerBuilder, McpResult,
    SessionContext,
};

// HTTP server config (when available)
#[cfg(feature = "http")]
pub use crate::http::ServerConfig;

// Server trait interfaces
pub use crate::{
    McpTool, McpResource, McpPrompt,
};

// Essential async trait for implementations
pub use async_trait::async_trait;

// Common serde types for serialization
pub use serde::{Serialize, Deserialize};

// Additional commonly used types
pub use std::sync::Arc;
pub use tokio;