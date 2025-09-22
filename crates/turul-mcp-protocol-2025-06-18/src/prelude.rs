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

// Resource traits
pub use crate::resources::{
    HasResourceMetadata, HasResourceUri, HasResourceDescription,
    HasResourceMimeType, HasResourceSize, HasResourceAnnotations,
    HasResourceMeta, ResourceDefinition, Resource, ResourceContent,
    ResourceTemplate, ListResourcesParams, ReadResourceParams,
};

// Prompt traits  
pub use crate::prompts::{
    HasPromptMetadata, HasPromptDescription, HasPromptArguments,
    HasPromptAnnotations, HasPromptMeta, PromptDefinition,
    Prompt, PromptMessage, PromptArgument, GetPromptParams,
};

// Tool traits
pub use crate::tools::{
    HasBaseMetadata, HasDescription, HasInputSchema, HasOutputSchema,
    HasAnnotations, HasToolMeta, ToolDefinition, Tool, ToolResult,
    CallToolParams, CallToolRequest, CallToolResult,
};

// Notification types (using specific structs that exist)
pub use crate::notifications::{
    Notification, NotificationParams, 
    ProgressNotification, ProgressNotificationParams, 
    ResourceUpdatedNotification, ResourceUpdatedNotificationParams,
    LoggingMessageNotification, LoggingMessageNotificationParams,
};

// Root traits
pub use crate::roots::{
    HasRootMetadata, HasRootPermissions, HasRootFiltering, HasRootAnnotations,
    RootDefinition
};

// Common types
pub use crate::{McpError, McpResult};
pub use crate::meta::{Annotations, Cursor};
pub use crate::json_rpc::{JsonRpcRequest, JsonRpcResponse, JsonRpcNotification};

// Common external types that are frequently used
pub use serde_json::{json, Value};
pub use std::collections::HashMap;