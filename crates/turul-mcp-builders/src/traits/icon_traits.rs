//! Framework trait for MCP icon support
//!
//! **IMPORTANT**: This is a framework feature, NOT part of the MCP specification.
//! Icons are supported on Tool, Resource, Prompt, ResourceTemplate, and Implementation
//! types as of MCP 2025-11-25.

use turul_mcp_protocol::icons::Icon;

/// Provides optional icons for MCP types.
///
/// Implement this trait to associate icons with your tool, resource, or prompt.
/// Icons are display hints — most implementations do not need icons.
///
/// ```rust
/// use turul_mcp_protocol::icons::Icon;
/// use turul_mcp_builders::prelude::*;
///
/// struct MyTool;
///
/// impl HasIcons for MyTool {
///     fn icons(&self) -> Option<&Vec<Icon>> {
///         None // No icons by default
///     }
/// }
/// ```
pub trait HasIcons {
    /// Returns the icons a client may display for this item, or `None` when it has none.
    fn icons(&self) -> Option<&Vec<Icon>> {
        None
    }
}
