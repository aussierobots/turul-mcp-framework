//! Framework trait for MCP tool execution configuration
//!
//! **IMPORTANT**: This is a framework feature, NOT part of the MCP specification.
//! The execution field on Tool (MCP 2025-11-25) declares per-tool task support.
//! In MCP 2026-07-28 tasks moved to the turul-mcp-ext-tasks extension, so the
//! core spec has no execution field; `HasExecution` is then a marker the derive
//! macro can implement uniformly across both specs.

#[cfg(feature = "protocol-2025-11-25")]
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
#[cfg(feature = "protocol-2025-11-25")]
pub trait HasExecution {
    fn execution(&self) -> Option<ToolExecution> {
        None
    }
}

/// Marker trait under MCP 2026-07-28 — the core spec has no Tool execution field
/// (tasks live in the turul-mcp-ext-tasks extension). It exists so the derive
/// macro's `impl HasExecution for T {}` compiles identically under both specs.
#[cfg(feature = "protocol-2026-07-28")]
pub trait HasExecution {}
