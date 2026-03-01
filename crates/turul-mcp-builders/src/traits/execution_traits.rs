//! Framework trait for MCP tool execution configuration
//!
//! **IMPORTANT**: This is a framework feature, NOT part of the MCP specification.
//! The execution field on Tool (MCP 2025-11-25) declares per-tool task support.

use turul_mcp_protocol::tools::ToolExecution;

/// Execution trait - provides optional execution configuration for MCP tools
///
/// Implement this trait to declare per-tool task support (`taskSupport`).
/// Most tools do not need this — the default returns `None`.
///
/// ```rust
/// use turul_mcp_protocol::tools::{ToolExecution, TaskSupport};
/// use turul_mcp_builders::prelude::*;
///
/// struct MyTool;
///
/// impl HasExecution for MyTool {
///     fn execution(&self) -> Option<ToolExecution> {
///         Some(ToolExecution {
///             task_support: Some(TaskSupport::Optional),
///         })
///     }
/// }
/// ```
pub trait HasExecution {
    fn execution(&self) -> Option<ToolExecution> {
        None
    }
}
