//! # Model Context Protocol (MCP) - Version Alias
//!
//! **The official Rust implementation of the Model Context Protocol specification.**
//!
//! This crate is a thin alias that re-exports exactly one versioned MCP protocol
//! crate, selected by a mutually-exclusive Cargo feature:
//!
//! - **`protocol-2026-07-28`** (default) — the 2026-07-28 stateless core.
//! - **`protocol-2025-11-25`** (opt-in) — the previous 2025-11-25 spec. Enable with
//!   `--no-default-features --features protocol-2025-11-25`.
//!
//! Exactly one must be active; enabling both (or neither) is a compile error.
//!
//! Flipping the default to 2026-07-28 is the 0.4 cutover. This alias is the
//! transition mechanism for 0.4; the long-term direction is for framework crates to
//! depend on the versioned protocol crates directly and retire the alias in 0.5.
//!
//! [![Crates.io](https://img.shields.io/crates/v/turul-mcp-protocol.svg)](https://crates.io/crates/turul-mcp-protocol)
//! [![Documentation](https://docs.rs/turul-mcp-protocol/badge.svg)](https://docs.rs/turul-mcp-protocol)
//! [![License](https://img.shields.io/crates/l/turul-mcp-protocol.svg)](https://github.com/aussierobots/turul-mcp-framework/blob/main/LICENSE)
//!
//! ## Installation
//!
//! ```toml
//! [dependencies]
//! turul-mcp-protocol = "0.4"  # default: protocol-2026-07-28
//! ```
//!
//! ## Quick Start
//!
//! ```rust
//! use turul_mcp_protocol::prelude::*;
//!
//! // Create core MCP types — same surface under either spec.
//! let tool = Tool::new("calculator", ToolSchema::object());
//! let resource = Resource::new("file://data.json", "data");
//! let prompt = Prompt::new("code_review");
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
//! | Feature | MCP Spec | Implementation Crate |
//! |---------|----------|---------------------|
//! | `protocol-2026-07-28` (default) | `2026-07-28` | `turul-mcp-protocol-2026-07-28` |
//! | `protocol-2025-11-25` (opt-in) | `2025-11-25` | `turul-mcp-protocol-2025-11-25` |

// Exactly one protocol-<date> feature must be active.
#[cfg(all(feature = "protocol-2025-11-25", feature = "protocol-2026-07-28"))]
compile_error!(
    "turul-mcp-protocol: features `protocol-2025-11-25` and `protocol-2026-07-28` \
     are mutually exclusive — a build re-exports exactly one MCP spec. Enable one."
);
#[cfg(not(any(feature = "protocol-2025-11-25", feature = "protocol-2026-07-28")))]
compile_error!(
    "turul-mcp-protocol: enable exactly one of `protocol-2025-11-25` (default) or \
     `protocol-2026-07-28`. If you used `--no-default-features`, add one explicitly."
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
