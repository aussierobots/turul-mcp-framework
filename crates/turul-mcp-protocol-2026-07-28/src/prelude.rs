//! Prelude module for common MCP protocol imports
//!
//! This module provides a convenient way to import the most commonly used
//! types and traits from the MCP protocol crate.
//!
//! # Usage
//!
//! ```rust,no_run
//! use turul_mcp_protocol_2026_07_28::prelude::*;
//!
//! // Now you have access to all common MCP types and traits
//! ```

// Shared pagination params (used by all `PaginatedRequest` extenders).
pub use crate::json_rpc::PaginatedRequestParams;

// Resource types (spec-pure structs only)
pub use crate::resources::{
    ReadResourceRequestParams, Resource, ResourceContent, ResourceTemplate,
};

// Prompt types (spec-pure structs only)
pub use crate::prompts::{ContentBlock, GetPromptRequestParams, Prompt, PromptArgument, PromptMessage};

// Tool types (spec-pure structs only)
pub use crate::tools::{
    CallToolRequestParams, CallToolRequest, CallToolResult, Tool, ToolResult, ToolSchema,
};

// Notification types (using specific structs that exist).
pub use crate::notifications::{
    Notification, NotificationParams, ProgressNotification, ProgressNotificationParams,
    ResourceUpdatedNotification, ResourceUpdatedNotificationParams,
};
// SEP-2577-deprecated re-exports kept available during the migration window.
#[allow(deprecated)]
pub use crate::notifications::{LoggingMessageNotification, LoggingMessageNotificationParams};

// Root types (spec-pure structs only). `ListRootsRequest.params?` is the
// standard `RequestParams` (already exported above), no roots-specific params
// struct. **Deprecated** per SEP-2577 in DRAFT-2026-v1 — re-exported for the
// migration window; new code should use tool parameters / resource URIs / server
// configuration instead.
#[allow(deprecated)]
pub use crate::roots::Root;

// Sampling types — **deprecated** per SEP-2577; re-exported during migration window.
#[allow(deprecated)]
pub use crate::sampling::{CreateMessageResult, SamplingMessage};

// Completion types (spec-pure structs only)
pub use crate::completion::{CompleteRequestParams, CompleteResult, CompletionReference};

// Capability types (initialize handshake removed in DRAFT-2026-v1 stateless core)
pub use crate::McpVersion;
pub use crate::icons::Icon;
pub use crate::initialize::{ClientCapabilities, Implementation};

// Common types — wire envelopes come from `turul-rpc` (re-exported at crate root).
pub use crate::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
pub use crate::meta::{Annotations, Cursor};
pub use crate::{McpError, McpResult};

// Common external types that are frequently used
pub use serde_json::{Value, json};
pub use std::collections::HashMap;
