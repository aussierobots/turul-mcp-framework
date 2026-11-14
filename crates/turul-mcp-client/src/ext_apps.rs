//! MCP Apps (`io.modelcontextprotocol/ui`) for host applications: read the
//! `_meta.ui` a server attaches and split app-only tools from the agent's.
//! Declare support with [`DeclaredCapabilities::ext_apps`](crate::config::DeclaredCapabilities::ext_apps).
//!
//! The client renders nothing. Keeping app-only tools from the model and
//! rejecting app calls to tools without `"app"` stay the host's job
//! (Apps spec §Visibility); [`partition_tools`] and
//! [`UiToolMeta::is_app_callable`] give it what it needs.

use turul_mcp_protocol_2025_11_25::ResourceContent;
use turul_mcp_protocol_2025_11_25::tools::Tool;

pub use turul_mcp_ext_apps::{UiResourceMeta, UiToolMeta, UiToolVisibility};

/// A tool's `_meta.ui`: `Ok(None)` when absent, an error when malformed.
pub fn tool_ui_meta(tool: &Tool) -> Result<Option<UiToolMeta>, serde_json::Error> {
    match &tool.meta {
        Some(meta) => UiToolMeta::from_meta(meta),
        None => Ok(None),
    }
}

/// A `resources/read` content item's `_meta.ui`: `Ok(None)` when absent, an
/// error when malformed.
pub fn resource_ui_meta(
    content: &ResourceContent,
) -> Result<Option<UiResourceMeta>, serde_json::Error> {
    let meta = match content {
        ResourceContent::Text(t) => &t.meta,
        ResourceContent::Blob(b) => &b.meta,
    };
    match meta {
        Some(meta) => UiResourceMeta::from_meta(meta),
        None => Ok(None),
    }
}

/// Tools split by who may see them.
#[derive(Debug, Clone, Default)]
pub struct ToolsByVisibility {
    /// Tools the agent may see: no `_meta.ui`, or `visibility` absent or
    /// including `"model"`.
    pub model_visible: Vec<Tool>,
    /// Tools only an app may call: `visibility` without `"model"`.
    pub app_only: Vec<Tool>,
}

/// Split tools into model-visible and app-only. A tool with malformed
/// `_meta.ui` fails the split rather than landing in either list.
pub fn partition_tools(tools: Vec<Tool>) -> Result<ToolsByVisibility, serde_json::Error> {
    let mut split = ToolsByVisibility::default();
    for tool in tools {
        let model_visible = tool_ui_meta(&tool)?.is_none_or(|ui| ui.is_model_visible());
        if model_visible {
            split.model_visible.push(tool);
        } else {
            split.app_only.push(tool);
        }
    }
    Ok(split)
}
