//! # Model Context Protocol (MCP) - Current Version
//!
//! **The official Rust implementation of the Model Context Protocol specification.**
//!
//! This crate provides a stable API that aliases the current version of the MCP specification,
//! ensuring your code stays up-to-date with the latest protocol version while maintaining
//! compatibility. Currently implements **MCP 2025-11-25** specification.
//!
//! [![Crates.io](https://img.shields.io/crates/v/turul-mcp-protocol.svg)](https://crates.io/crates/turul-mcp-protocol)
//! [![Documentation](https://docs.rs/turul-mcp-protocol/badge.svg)](https://docs.rs/turul-mcp-protocol)
//! [![License](https://img.shields.io/crates/l/turul-mcp-protocol.svg)](https://github.com/aussierobots/turul-mcp-framework/blob/main/LICENSE)
//!
//! ## Installation
//!
//! ```toml
//! [dependencies]
//! turul-mcp-protocol = "0.3"
//! ```
//!
//! ## Quick Start
//!
//! ```rust
//! use turul_mcp_protocol::prelude::*;
//!
//! // Create tools, resources, prompts
//! let tool = Tool::new("calculator", ToolSchema::object());
//! let resource = Resource::new("file://data.json", "data");
//! let prompt = Prompt::new("code_review");
//!
//! // Handle requests and responses
//! let request = InitializeRequest::new(
//!     McpVersion::CURRENT,
//!     ClientCapabilities::default(),
//!     Implementation::new("my-client", "1.0.0")
//! );
//! ```
//!
//! ## Protocol Types
//!
//! This crate provides all core MCP types:
//!
//! - **Tools**: `Tool`, `CallToolRequest`, `CallToolResult`
//! - **Resources**: `Resource`, `ReadResourceRequest`, `ResourceContent`
//! - **Prompts**: `Prompt`, `GetPromptRequest`, `PromptMessage`
//! - **Notifications**: `ProgressNotification`, `LoggingMessage`
//! - **Protocol**: `InitializeRequest`, `McpVersion`, `ServerCapabilities`
//! - **Errors**: `McpError`, `JsonRpcError`, error codes
//!
//! ## Use Cases
//!
//! - **MCP Server Development**: Use with [`turul-mcp-server`](https://crates.io/crates/turul-mcp-server)
//! - **MCP Client Development**: Use with [`turul-mcp-client`](https://crates.io/crates/turul-mcp-client)
//! - **Protocol Parsing**: Direct protocol message handling
//! - **Type Definitions**: Reference implementation for MCP types
//!
//! ## Related Crates
//!
//! - [`turul-mcp-server`](https://crates.io/crates/turul-mcp-server) - High-level server framework
//! - [`turul-mcp-client`](https://crates.io/crates/turul-mcp-client) - Client library
//! - [`turul-mcp-derive`](https://crates.io/crates/turul-mcp-derive) - Procedural macros
//!
//! ## Version Mapping
//!
//! | This Crate | MCP Spec | Implementation Crate |
//! |------------|----------|---------------------|
//! | `0.3.x` | `2025-11-25` | `turul-mcp-protocol-2025-11-25` |
//!
//! Currently aliases: `turul-mcp-protocol-2025-11-25`

// Re-export the current MCP protocol version
pub use turul_mcp_protocol_2025_11_25::*;

// Explicitly re-export the prelude module for convenient imports
pub mod prelude {
    pub use turul_mcp_protocol_2025_11_25::prelude::*;
}

/// The current MCP protocol version implemented by this crate
pub const CURRENT_VERSION: &str = MCP_VERSION;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_version() {
        assert_eq!(CURRENT_VERSION, "2025-11-25");
        assert_eq!(MCP_VERSION, "2025-11-25");
    }

    #[test]
    fn test_version_parsing() {
        let version = "2025-11-25".parse::<McpVersion>().unwrap();
        assert_eq!(version, McpVersion::V2025_11_25);
    }

    #[test]
    fn test_re_exports_work() {
        // Test that we can create basic types
        let _impl = Implementation::new("test", "1.0.0");
        let _capabilities = ClientCapabilities::default();
        let _tool = Tool::new("test", ToolSchema::object());

        // If this compiles, the re-exports are working
    }

    #[test]
    fn test_prelude_works() {
        use crate::prelude::*;

        // Test that prelude types are available
        let _tool = Tool::new("test", ToolSchema::object());
        let _resource = Resource::new("test://resource", "test_resource");
        let _prompt = Prompt::new("test_prompt");
        let _error = McpError::tool_execution("test error");

        // If this compiles, the prelude is working
    }
}
