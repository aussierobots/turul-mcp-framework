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
    HasResourceAnnotations, HasResourceDescription, HasResourceMeta, HasResourceMetadata,
    HasResourceMimeType, HasResourceSize, HasResourceUri, ListResourcesParams, ReadResourceParams,
    Resource, ResourceContent, ResourceDefinition, ResourceTemplate,
};

// Prompt traits
pub use crate::prompts::{
    ContentBlock, GetPromptParams, HasPromptAnnotations, HasPromptArguments, HasPromptDescription,
    HasPromptMeta, HasPromptMetadata, Prompt, PromptArgument, PromptDefinition, PromptMessage,
};

// Tool traits
pub use crate::tools::{
    CallToolParams, CallToolRequest, CallToolResult, HasAnnotations, HasBaseMetadata,
    HasDescription, HasInputSchema, HasOutputSchema, HasToolMeta, Tool, ToolDefinition, ToolResult,
    ToolSchema,
};

// Notification types (using specific structs that exist)
pub use crate::notifications::{
    LoggingMessageNotification, LoggingMessageNotificationParams, Notification, NotificationParams,
    ProgressNotification, ProgressNotificationParams, ResourceUpdatedNotification,
    ResourceUpdatedNotificationParams,
};

// Root traits
pub use crate::roots::{
    HasRootAnnotations, HasRootFiltering, HasRootMetadata, HasRootPermissions, RootDefinition,
};

// Sampling types
pub use crate::sampling::{
    CreateMessageResult, HasSamplingConfig, HasSamplingContext, Role, SamplingDefinition,
    SamplingMessage,
};

// Completion types
pub use crate::completion::{
    CompletionDefinition, HasCompletionContext, HasCompletionHandling, HasCompletionMetadata,
};

// Logging types
pub use crate::logging::{HasLogLevel, HasLoggingMetadata, LoggerDefinition};

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
