//! # Model Context Protocol (MCP) - Current Version
//!
//! This crate provides an alias to the current version of the Model Context Protocol
//! implementation. It re-exports the latest stable MCP specification.
//!
//! Currently aliases: `mcp-protocol-2025-06-18`
//!
//! ## Usage
//!
//! ```rust
//! use mcp_protocol::{
//!     McpVersion, InitializeRequest, InitializeResponse,
//!     Tool, CallToolRequest, CallToolResponse
//! };
//! ```

// Re-export the current MCP protocol version
pub use mcp_protocol_2025_06_18::*;

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
        let version = McpVersion::from_str("2025-06-18").unwrap();
        assert_eq!(version, McpVersion::V2025_06_18);
    }

    #[test]
    fn test_re_exports_work() {
        // Test that we can create basic types
        let _impl = Implementation::new("test", "1.0.0");
        let _capabilities = ClientCapabilities::default();
        let _tool = Tool::new("test", ToolSchema::object());
        
        // If this compiles, the re-exports are working
        assert!(true);
    }
}
