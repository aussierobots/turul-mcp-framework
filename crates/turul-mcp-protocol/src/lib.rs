//! # Model Context Protocol (MCP) - Current Version
//!
//! This crate provides an alias to the current version of the Model Context Protocol
//! implementation. It re-exports the latest stable MCP specification.
//!
//! Currently aliases: `turul-mcp-protocol-2025-06-18`
//!
//! ## Usage
//!
//! ```rust
//! use turul_mcp_protocol::{
//!     McpVersion, InitializeRequest, InitializeResult,
//!     Tool, CallToolRequest, CallToolResult
//! };
//!
//! // Or use the prelude for common types
//! use turul_mcp_protocol::prelude::*;
//! ```

// Re-export the current MCP protocol version
pub use turul_mcp_protocol_2025_06_18::*;

// Explicitly re-export the prelude module for convenient imports
pub mod prelude {
    pub use turul_mcp_protocol_2025_06_18::prelude::*;
}

/// The current MCP protocol version implemented by this crate
pub const CURRENT_VERSION: &str = MCP_VERSION;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_version() {
        assert_eq!(CURRENT_VERSION, "2025-06-18");
        assert_eq!(MCP_VERSION, "2025-06-18");
    }

    #[test]
    fn test_version_parsing() {
        let version = "2025-06-18".parse::<McpVersion>().unwrap();
        assert_eq!(version, McpVersion::V2025_06_18);
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
