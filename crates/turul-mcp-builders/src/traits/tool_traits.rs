//! Framework traits for MCP tool construction
//!
//! **IMPORTANT**: These are framework features, NOT part of the MCP specification.
//! The MCP specification defines concrete types only.

use serde_json::Value;
use std::collections::HashMap;

use turul_mcp_protocol::tools::ToolAnnotations;
use turul_mcp_protocol::{Tool, ToolSchema};

/// Provides the base metadata (`name`, `title`), mirroring `BaseMetadata` in the MCP schema.
pub trait HasBaseMetadata {
    /// Returns the programmatic identifier, used as the fallback display name.
    fn name(&self) -> &str;

    /// Returns the human-readable display name for UI contexts.
    fn title(&self) -> Option<&str> {
        None
    }
}

/// Provides the optional tool description.
pub trait HasDescription {
    /// Returns the human-readable tool description, or `None` when it has none.
    fn description(&self) -> Option<&str> {
        None
    }
}

/// Provides the tool input schema.
pub trait HasInputSchema {
    /// Returns the JSON Schema describing the tool's input arguments.
    fn input_schema(&self) -> &ToolSchema;
}

/// Provides the optional tool output schema.
pub trait HasOutputSchema {
    /// Returns the JSON Schema describing the tool's structured output, or `None` when it declares none.
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}

/// Provides the optional tool annotations.
pub trait HasAnnotations {
    /// Returns the tool annotations (behavior hints), or `None` when it has none.
    fn annotations(&self) -> Option<&ToolAnnotations> {
        None
    }
}

/// Provides the optional tool-specific `_meta`, separate from the RPC-level `_meta`.
pub trait HasToolMeta {
    /// Returns the tool-specific `_meta` map, or `None` when it has none.
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

/// Defines a complete tool, composed from fine-grained traits.
///
/// This trait represents a complete MCP tool that can be registered with a server
/// and invoked by clients. When you implement the required metadata traits, you automatically
/// get `ToolDefinition` for free via blanket implementation.
#[cfg(feature = "protocol-2025-11-25")]
pub trait ToolDefinition:
    HasBaseMetadata +           // name, title
    HasDescription +            // description
    HasInputSchema +            // inputSchema
    HasOutputSchema +           // outputSchema
    HasAnnotations +            // annotations
    HasToolMeta +               // _meta (tool-specific)
    super::icon_traits::HasIcons + // icons
    super::execution_traits::HasExecution + // execution.taskSupport
    Send +
    Sync
{
    /// Returns the display name, preferring `title` over `name`.
    fn display_name(&self) -> &str {
        if let Some(title) = self.title() {
            title
        } else {
            self.name()
        }
    }

    /// Converts this definition to a protocol `Tool` for serialization.
    fn to_tool(&self) -> Tool {
        Tool {
            name: self.name().to_string(),
            title: self.title().map(String::from),
            description: self.description().map(String::from),
            input_schema: self.input_schema().clone(),
            output_schema: self.output_schema().cloned(),
            annotations: self.annotations().cloned(),
            execution: self.execution(),
            icons: self.icons().cloned(),
            meta: self.tool_meta().cloned(),
        }
    }
}

#[cfg(feature = "protocol-2025-11-25")]
impl<T> ToolDefinition for T
where
    T: HasBaseMetadata
        + HasDescription
        + HasInputSchema
        + HasOutputSchema
        + HasAnnotations
        + HasToolMeta
        + super::icon_traits::HasIcons
        + super::execution_traits::HasExecution
        + Send
        + Sync,
{
    // Default implementations provided by trait definition
}

/// Defines a complete tool, composed from fine-grained traits.
#[cfg(feature = "protocol-2026-07-28")]
pub trait ToolDefinition:
    HasBaseMetadata +           // name, title
    HasDescription +            // description
    HasInputSchema +            // inputSchema
    HasOutputSchema +           // outputSchema
    HasAnnotations +            // annotations
    HasToolMeta +               // _meta (tool-specific)
    super::icon_traits::HasIcons + // icons
    Send +
    Sync
{
    /// Returns the display name, preferring `title` over `name`.
    fn display_name(&self) -> &str {
        if let Some(title) = self.title() {
            title
        } else {
            self.name()
        }
    }

    /// Converts this definition to a protocol `Tool` for serialization.
    fn to_tool(&self) -> Tool {
        Tool {
            name: self.name().to_string(),
            title: self.title().map(String::from),
            description: self.description().map(String::from),
            input_schema: self.input_schema().clone(),
            // The trait's object-rooted ToolSchema and the 2026 wire type
            // (free-form ToolOutputSchema, flattened map) carry the same JSON;
            // a serde round-trip is the lossless bridge between them.
            output_schema: self.output_schema().and_then(|schema| {
                serde_json::to_value(schema)
                    .ok()
                    .and_then(|v| serde_json::from_value(v).ok())
            }),
            annotations: self.annotations().cloned(),
            icons: self.icons().cloned(),
            meta: self.tool_meta().cloned(),
        }
    }
}

#[cfg(feature = "protocol-2026-07-28")]
impl<T> ToolDefinition for T
where
    T: HasBaseMetadata
        + HasDescription
        + HasInputSchema
        + HasOutputSchema
        + HasAnnotations
        + HasToolMeta
        + super::icon_traits::HasIcons
        + Send
        + Sync,
{
    // Default implementations provided by trait definition
}
