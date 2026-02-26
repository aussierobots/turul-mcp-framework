//! Prelude for turul-mcp-builders
//!
//! Convenient imports for framework traits and runtime builders.
//!
//! # Usage
//!
//! ```rust
//! use turul_mcp_builders::prelude::*;
//!
//! // Now you have access to:
//! // - All framework traits (ToolDefinition, HasBaseMetadata, etc.)
//! // - All runtime builders (ToolBuilder, ResourceBuilder, etc.)
//! ```

// Re-export all framework traits
pub use crate::traits::*;

// Re-export all builders
pub use crate::completion::CompletionBuilder;
pub use crate::elicitation::ElicitationBuilder;
pub use crate::logging::LoggingBuilder;
pub use crate::message::MessageBuilder;
pub use crate::notification::NotificationBuilder;
pub use crate::prompt::PromptBuilder;
pub use crate::resource::ResourceBuilder;
pub use crate::root::RootBuilder;
pub use crate::tool::{DynamicToolFn, ToolBuilder};

// Re-export commonly used protocol types for convenience
pub use turul_mcp_protocol::{
    McpError, McpResult, Prompt, PromptMessage, Resource, ResourceContent, Tool, ToolSchema,
};

// Re-export common std/external types
pub use serde_json::{Value, json};
pub use std::collections::HashMap;
