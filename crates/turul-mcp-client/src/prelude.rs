//! # MCP Client Prelude
//!
//! This module provides convenient re-exports of the most commonly used types
//! and traits from the MCP client library.
//!
//! ```rust
//! use turul_mcp_client::prelude::*;
//! ```

// Core client types
pub use crate::client::{McpClient, McpClientBuilder};
pub use crate::config::{ClientConfig, RetryConfig, TimeoutConfig};
pub use crate::error::{McpClientError, McpClientResult};
pub use crate::session::{SessionInfo, SessionManager, SessionState};

// Transport types
pub use crate::transport::{Transport, TransportType};

// Re-export protocol types for convenience
pub use turul_mcp_protocol::prelude::*;

// Standard library types commonly used with MCP
pub use std::time::Duration;