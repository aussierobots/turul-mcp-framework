//! Framework traits for MCP tool construction
//!
//! **IMPORTANT**: These are framework features, NOT part of the MCP specification.
//! The MCP specification defines concrete types only.

use std::collections::HashMap;
use serde_json::Value;

// Import protocol types (spec-defined)
use turul_mcp_protocol::{Tool, ToolSchema};
use turul_mcp_protocol::tools::ToolAnnotations;

/// Base metadata trait - matches TypeScript BaseMetadata interface
pub trait HasBaseMetadata {
    /// Programmatic identifier (fallback display name)
    fn name(&self) -> &str;

    /// Human-readable display name (UI contexts)
    fn title(&self) -> Option<&str> {
        None
    }
}

/// Tool description trait
pub trait HasDescription {
    fn description(&self) -> Option<&str> {
        None
    }
}

/// Input schema trait
pub trait HasInputSchema {
    fn input_schema(&self) -> &ToolSchema;
}

/// Output schema trait
pub trait HasOutputSchema {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}

/// Annotations trait
pub trait HasAnnotations {
    fn annotations(&self) -> Option<&ToolAnnotations> {
        None
    }
}

/// Tool-specific meta trait (separate from RPC _meta)
pub trait HasToolMeta {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

/// Complete tool definition - composed from fine-grained traits
///
/// This trait represents a complete MCP tool that can be registered with a server
/// and invoked by clients. When you implement the required metadata traits, you automatically
/// get `ToolDefinition` for free via blanket implementation.
pub trait ToolDefinition:
    HasBaseMetadata +           // name, title
    HasDescription +            // description
    HasInputSchema +            // inputSchema
    HasOutputSchema +           // outputSchema
    HasAnnotations +            // annotations
    HasToolMeta +               // _meta (tool-specific)
    Send +
    Sync
{
    /// Display name precedence: title > annotations.title > name (matches TypeScript spec)
    fn display_name(&self) -> &str {
        if let Some(title) = self.title() {
            title
        } else if let Some(annotations) = self.annotations() {
            if let Some(title) = &annotations.title {
                title
            } else {
                self.name()
            }
        } else {
            self.name()
        }
    }

    /// Convert to concrete Tool struct for protocol serialization
    fn to_tool(&self) -> Tool {
        Tool {
            name: self.name().to_string(),
            title: self.title().map(String::from),
            description: self.description().map(String::from),
            input_schema: self.input_schema().clone(),
            output_schema: self.output_schema().cloned(),
            annotations: self.annotations().cloned(),
            meta: self.tool_meta().cloned(),
        }
    }
}

/// Blanket implementation: any type implementing all required traits gets ToolDefinition
impl<T> ToolDefinition for T
where
    T: HasBaseMetadata
        + HasDescription
        + HasInputSchema
        + HasOutputSchema
        + HasAnnotations
        + HasToolMeta
        + Send
        + Sync,
{
    // Default implementations provided by trait definition
}
