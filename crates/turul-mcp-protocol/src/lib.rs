//! MCP wire types for building servers and clients; selects one versioned protocol crate.
//!
//! Most users want [`turul-mcp-server`](https://docs.rs/turul-mcp-server), which
//! re-exports what it needs from here. Depend on this crate directly only to parse
//! or produce MCP messages without the server framework.
//!
//! By default it re-exports `turul-mcp-protocol-2026-07-28` (MCP 2026-07-28). The
//! 2025-11-25 spec is an opt-in lane selected by a Cargo feature; the two are
//! mutually exclusive and exactly one must be active.
//!
//! # Example
//!
//! ```toml
//! [dependencies]
//! turul-mcp-protocol = "0.4"
//! ```
//!
//! ```rust
//! use turul_mcp_protocol::prelude::*;
//!
//! let tool = Tool::new("calculator", ToolSchema::object());
//! let resource = Resource::new("file://data.json", "data");
//! let prompt = Prompt::new("code_review");
//! ```
//!
//! # Types
//!
//! - Tools: [`Tool`], [`CallToolRequest`], [`CallToolResult`]
//! - Resources: [`Resource`], [`ReadResourceRequest`], [`ResourceContent`]
//! - Prompts: [`Prompt`], [`GetPromptRequest`], [`PromptMessage`]
//! - Notifications: [`ProgressNotification`]
//! - Negotiation and errors: [`McpVersion`], [`ServerCapabilities`], [`McpError`]
//!
//! # Features
//!
//! | Feature | MCP spec | Crate re-exported |
//! |---|---|---|
//! | `protocol-2026-07-28` (default) | 2026-07-28 | `turul-mcp-protocol-2026-07-28` |
//! | `protocol-2025-11-25` | 2025-11-25 | `turul-mcp-protocol-2025-11-25` |
//!
//! Select the opt-in lane with `--no-default-features --features protocol-2025-11-25`.
//!
//! # Where next
//!
//! - [`turul-mcp-server`](https://docs.rs/turul-mcp-server): build a server
//! - [`turul-mcp-client`](https://docs.rs/turul-mcp-client): build a client
//! - [`turul-mcp-protocol-2026-07-28`](https://docs.rs/turul-mcp-protocol-2026-07-28): full type reference

// Exactly one protocol-<date> feature must be active.
#[cfg(all(feature = "protocol-2025-11-25", feature = "protocol-2026-07-28"))]
compile_error!(
    "turul-mcp-protocol: features `protocol-2025-11-25` and `protocol-2026-07-28` \
     are mutually exclusive — a build re-exports exactly one MCP spec. Enable one."
);
#[cfg(not(any(feature = "protocol-2025-11-25", feature = "protocol-2026-07-28")))]
compile_error!(
    "turul-mcp-protocol: enable exactly one of `protocol-2026-07-28` (default) or \
     `protocol-2025-11-25`. If you used `--no-default-features`, add one explicitly."
);

// Re-export the selected MCP protocol version.
#[cfg(feature = "protocol-2025-11-25")]
pub use turul_mcp_protocol_2025_11_25::*;
#[cfg(feature = "protocol-2026-07-28")]
pub use turul_mcp_protocol_2026_07_28::*;

// Explicitly re-export the prelude module for convenient imports.
pub mod prelude {
    #[cfg(feature = "protocol-2025-11-25")]
    pub use turul_mcp_protocol_2025_11_25::prelude::*;
    #[cfg(feature = "protocol-2026-07-28")]
    pub use turul_mcp_protocol_2026_07_28::prelude::*;
}

/// The current MCP protocol version implemented by this crate
pub const CURRENT_VERSION: &str = MCP_VERSION;

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "protocol-2026-07-28")]
    #[test]
    fn test_current_version() {
        assert_eq!(CURRENT_VERSION, "2026-07-28");
        assert_eq!(MCP_VERSION, "2026-07-28");
    }

    #[cfg(feature = "protocol-2025-11-25")]
    #[test]
    fn test_current_version() {
        assert_eq!(CURRENT_VERSION, "2025-11-25");
        assert_eq!(MCP_VERSION, "2025-11-25");
    }

    #[cfg(feature = "protocol-2026-07-28")]
    #[test]
    fn test_version_parsing() {
        let version = "2026-07-28".parse::<McpVersion>().unwrap();
        assert_eq!(version, McpVersion::V2026_07_28);
    }

    #[cfg(feature = "protocol-2025-11-25")]
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
