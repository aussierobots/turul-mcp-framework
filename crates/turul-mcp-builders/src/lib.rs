//! Builders that create MCP tools, resources and prompts at runtime, for definitions
//! that are only known when the program runs, such as ones read from configuration.
//!
//! Most users want [`turul-mcp-server`](https://docs.rs/turul-mcp-server), which
//! re-exports these builders. For tools known at compile time, use its
//! `#[mcp_tool]` macro instead: it is less code and derives the schema from your types.
//!
//! # Example
//!
//! ```toml
//! [dependencies]
//! turul-mcp-builders = "0.4"
//! serde_json = "1"
//! tokio = { version = "1", features = ["macros", "rt"] }
//! ```
//!
//! ```rust
//! use serde_json::json;
//! use turul_mcp_builders::{HasBaseMetadata, ToolBuilder};
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let tool = ToolBuilder::new("add")
//!     .description("Add two numbers")
//!     .number_param("a", "First number")
//!     .number_param("b", "Second number")
//!     .execute(|args| async move {
//!         let a = args["a"].as_f64().ok_or("a must be a number")?;
//!         let b = args["b"].as_f64().ok_or("b must be a number")?;
//!         Ok(json!({"result": a + b}))
//!     })
//!     .build()?;
//!
//! assert_eq!(tool.name(), "add");
//! assert_eq!(tool.execute(json!({"a": 5.0, "b": 3.0})).await?, json!({"result": 8.0}));
//! # Ok(())
//! # }
//! ```
//!
//! [`ResourceBuilder`] and [`PromptBuilder`] work the same way. A builder's output
//! is registered on a server with `McpServer::builder().tool(...)`, `.resource(...)`
//! or `.prompt(...)`.
//!
//! # Features
//!
//! | Feature | Default | Enables |
//! |---|---|---|
//! | `protocol-2026-07-28` | yes | MCP 2026-07-28 |
//! | `protocol-2025-11-25` | no | MCP 2025-11-25 builders for sampling, completion and logging; use with `--no-default-features` |
//!
//! # Where next
//!
//! - [`ToolBuilder`] for every parameter and schema option
//! - [`turul-mcp-server`](https://docs.rs/turul-mcp-server) to serve what you build
//! - [Examples](https://github.com/aussierobots/turul-mcp-framework/tree/main/examples)

pub mod prelude;
pub mod protocol_impls;
pub mod traits;

#[cfg(feature = "protocol-2025-11-25")]
pub mod completion;
pub mod elicitation;
#[cfg(feature = "protocol-2025-11-25")]
pub mod logging;
#[cfg(feature = "protocol-2025-11-25")]
pub mod message;
pub mod notification;
pub mod prompt;
pub mod resource;
pub mod root;
pub mod tool;

// Schemars integration for JSON schema generation
pub mod schemars_helpers;
pub use schemars_helpers::{
    ToolSchemaExt, convert_value_to_json_schema, convert_value_to_json_schema_with_defs,
    resolve_local_refs, schemars_param_schema,
};

// Schema provider with automatic JsonSchema detection
// schema_provider module removed - schemars detection now happens directly in derive macros

// Re-export schemars for use in generated code
// This ensures schemars::schema_for!() works in user code without requiring
// users to add schemars as an explicit dependency
pub use schemars;

// Re-export all framework traits for convenience
pub use traits::*;

// Re-export all builders for convenience
/// Builder for completion provider configuration with sampling parameters
#[cfg(feature = "protocol-2025-11-25")]
pub use completion::CompletionBuilder;
#[cfg(feature = "protocol-2026-07-28")]
pub use elicitation::validate_elicit_content;
/// Builders for interactive data collection and result formatting
pub use elicitation::{ElicitResultBuilder, ElicitationBuilder};
/// Builders for structured logging messages and level configuration
#[cfg(feature = "protocol-2025-11-25")]
pub use logging::{LoggingBuilder, SetLevelBuilder};
/// Builder for constructing prompt messages with role and content
#[cfg(feature = "protocol-2025-11-25")]
pub use message::MessageBuilder;
/// Builders for various MCP notification types (progress, cancellation, resource updates)
pub use notification::{
    CancelledNotificationBuilder, NotificationBuilder, ProgressNotificationBuilder,
    ResourceUpdatedNotificationBuilder,
};
/// Builder for prompt templates with arguments and message composition
pub use prompt::PromptBuilder;
/// Builder for resource definitions with content and metadata
pub use resource::ResourceBuilder;
/// Builders for root directory listings and workspace notifications
pub use root::RootBuilder;
#[cfg(feature = "protocol-2025-11-25")]
pub use root::{ListRootsRequestBuilder, RootsNotificationBuilder};
/// Builder for executable tool definitions with parameters and handlers
pub use tool::ToolBuilder;

// Common types used across builders
/// JSON value type for dynamic content construction
pub use serde_json::{Value, json};
/// Hash map type for parameter collections and metadata
pub use std::collections::HashMap;

/// Adapts a property map (`name -> structured JsonSchema`) to the value type
/// `ToolSchema::with_properties` accepts for the active spec: the structured
/// `JsonSchema` under 2025-11-25, or its JSON projection under 2026-07-28,
/// whose properties hold arbitrary JSON Schema 2020-12 values.
#[cfg(feature = "protocol-2025-11-25")]
pub fn tool_props(
    p: std::collections::HashMap<String, turul_mcp_protocol::schema::JsonSchema>,
) -> std::collections::HashMap<String, turul_mcp_protocol::schema::JsonSchema> {
    p
}

/// Adapts a property map (`name -> structured JsonSchema`) to the value type
/// `ToolSchema::with_properties` accepts for the active spec: the structured
/// `JsonSchema` under 2025-11-25, or its JSON projection under 2026-07-28,
/// whose properties hold arbitrary JSON Schema 2020-12 values.
#[cfg(feature = "protocol-2026-07-28")]
pub fn tool_props(
    p: std::collections::HashMap<String, turul_mcp_protocol::schema::JsonSchema>,
) -> std::collections::HashMap<String, serde_json::Value> {
    p.into_iter()
        .map(|(k, v)| {
            (
                k,
                serde_json::to_value(v).unwrap_or(serde_json::Value::Null),
            )
        })
        .collect()
}

/// Reports whether a `ToolSchema` property value denotes an object-typed schema.
/// Accepts either form a property map holds across specs — a structured
/// `JsonSchema` (2025-11-25) or an arbitrary JSON Schema value (2026-07-28) —
/// by inspecting the serialized `"type"` field.
pub fn tool_prop_is_object<T: serde::Serialize>(prop: &T) -> bool {
    serde_json::to_value(prop)
        .ok()
        .and_then(|v| {
            v.get("type")
                .and_then(|t| t.as_str())
                .map(|s| s == "object")
        })
        .unwrap_or(false)
}
