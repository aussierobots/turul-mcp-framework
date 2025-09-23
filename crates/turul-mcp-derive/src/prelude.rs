//! # MCP Derive Prelude
//!
//! This module provides convenient re-exports of the most commonly used macros
//! from the MCP derive library.
//!
//! ```rust
//! use turul_mcp_derive::prelude::*;
//! ```

// Most commonly used derive macros
pub use crate::{McpTool, McpResource, McpPrompt, JsonSchema};

// Most commonly used attribute macros
pub use crate::{mcp_tool, mcp_resource, param};

// Most commonly used declarative macros
pub use crate::{tool, resource, schema_for};

// Additional commonly used derives
pub use crate::{McpNotification, McpCompletion, McpSampling};