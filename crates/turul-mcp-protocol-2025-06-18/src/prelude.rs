//! Prelude module for common MCP protocol imports
//!
//! This module provides a convenient way to import the most commonly used
//! types and traits from the MCP protocol crate.
//!
//! # Usage
//!
//! ```rust,no_run
//! use turul_mcp_protocol_2025_06_18::prelude::*;
//!
//! // Now you have access to all common MCP types and traits
//! ```

// Resource types (spec-pure structs only)
pub use crate::resources::{
    ListResourcesParams, ReadResourceParams, Resource, ResourceContent, ResourceTemplate,
};

// Prompt types (spec-pure structs only)
pub use crate::prompts::{
    ContentBlock, GetPromptParams, Prompt, PromptArgument, PromptMessage,
};

// Tool types (spec-pure structs only)
pub use crate::tools::{
    CallToolParams, CallToolRequest, CallToolResult, Tool, ToolResult, ToolSchema,
};

// Notification types (using specific structs that exist)
pub use crate::notifications::{
    LoggingMessageNotification, LoggingMessageNotificationParams, Notification, NotificationParams,
    ProgressNotification, ProgressNotificationParams, ResourceUpdatedNotification,
    ResourceUpdatedNotificationParams,
};

// Root types (spec-pure structs only)
pub use crate::roots::{
    ListRootsParams, Root,
};

// Sampling types (spec-pure structs only)
pub use crate::sampling::{
    CreateMessageResult, SamplingMessage,
};

// Completion types (spec-pure structs only)
pub use crate::completion::{
    CompleteParams, CompleteResult, CompletionReference,
};

// Initialize types
pub use crate::McpVersion;
pub use crate::initialize::{ClientCapabilities, Implementation, InitializeRequest};

// Common types
pub use crate::json_rpc::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
pub use crate::meta::{Annotations, Cursor};
pub use crate::{McpError, McpResult};

// Common external types that are frequently used
pub use serde_json::{Value, json};
pub use std::collections::HashMap;
