//! # Model Context Protocol (MCP) - 2025-11-25 Specification
//!
//! This crate provides a complete implementation of the Model Context Protocol (MCP)
//! specification version 2025-11-25. It includes all the types, traits, and utilities
//! needed to build MCP-compliant servers and clients.
//!
//! ## Features
//! - Complete MCP 2025-11-25 specification compliance
//! - Support for all MCP capabilities (tools, resources, prompts, etc.)
//! - Built on top of the turul-json-rpc-server foundation
//! - Support for streamable HTTP and _meta fields
//! - Progress tokens and cursor support
//! - Structured user elicitation via JSON Schema (form and URL modes)
//! - Meta field merging utilities for request/response round-tripping
//! - Task system (experimental) for long-running operations
//! - Icons on tools, resources, prompts, and implementations
//! - Tools in sampling requests
//!
//! ## Meta Field Usage
//!
//! The protocol supports rich `_meta` fields for pagination, progress tracking,
//! and custom metadata:
//!
//! ```rust
//! use turul_mcp_protocol_2025_11_25::meta::{Meta, Cursor};
//! use std::collections::HashMap;
//! use serde_json::{json, Value};
//!
//! // Create meta with pagination
//! let meta = Meta::with_pagination(
//!     Some(Cursor::new("next-page")),
//!     Some(100),
//!     true
//! );
//!
//! // Merge request extras while preserving structured fields
//! let mut request_extras = HashMap::new();
//! request_extras.insert("userContext".to_string(), json!("user_123"));
//! request_extras.insert("customField".to_string(), json!("custom_value"));
//!
//! let response_meta = meta.merge_request_extras(Some(&request_extras));
//! ```

pub mod completion;
pub mod content;
pub mod elicitation;
pub mod icons;
pub mod initialize;
pub mod json_rpc;
pub mod logging;
pub mod meta;
pub mod notifications;
pub mod param_extraction;
pub mod ping;
pub mod prelude;
pub mod prompts;
pub mod resources;
pub mod roots;
pub mod sampling;
pub mod schema;
pub mod tasks;
pub mod tools;
pub mod traits;
pub mod version;

// Re-export key content types for convenience
pub use content::{
    BlobResourceContents, ContentBlock, ResourceContents, ResourceReference, TextResourceContents,
};
// Re-export key meta types for convenience
pub use meta::{Annotations, Meta};

#[cfg(test)]
mod compliance_test;

// Re-export main types
pub use icons::{Icon, IconTheme};
pub use initialize::{
    ClientCapabilities, Implementation, InitializeRequest, InitializeResult, ServerCapabilities,
    TasksCancelCapabilities, TasksCapabilities, TasksListCapabilities, TasksRequestCapabilities,
    TasksToolCallCapabilities, TasksToolCapabilities,
};
pub use prompts::{
    GetPromptRequest, GetPromptResult, ListPromptsRequest, ListPromptsResult, Prompt,
    PromptArgument, PromptMessage,
};
pub use resources::{
    ListResourcesRequest, ListResourcesResult, ReadResourceRequest, ReadResourceResult, Resource,
    ResourceContent, ResourceSubscription, SubscribeRequest, UnsubscribeRequest,
};
pub use tasks::{
    CancelTaskParams, CancelTaskRequest, CancelTaskResult, CreateTaskResult, GetTaskParams,
    GetTaskPayloadParams, GetTaskPayloadRequest, GetTaskRequest, GetTaskResult, ListTasksParams,
    ListTasksRequest, ListTasksResult, Task, TaskMetadata, TaskStatus,
};
pub use tools::{
    CallToolRequest, CallToolResult, ListToolsRequest, ListToolsResult, TaskSupport, Tool,
    ToolExecution, ToolResult, ToolSchema,
};
pub use version::McpVersion;
// ResourceTemplate functionality is now part of resources module
// pub use resources::{ResourceTemplate, ListResourceTemplatesRequest, ListResourceTemplatesResult};
pub use elicitation::{
    ElicitAction, ElicitCreateParams, ElicitCreateRequest, ElicitResult, ElicitationBuilder,
    ElicitationSchema, PrimitiveSchemaDefinition, StringFormat,
};
pub use json_rpc::{
    JsonRpcError, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
    RequestParams, ResultWithMeta,
};
pub use meta::{
    Cursor as MetaCursor, PaginatedResponse, ProgressResponse, ProgressToken, WithMeta,
};
pub use notifications::{
    CancelledNotification, ElicitationCompleteNotification, InitializedNotification,
    LoggingMessageNotification, LoggingMessageNotificationParams, Notification, NotificationParams,
    ProgressNotification, ProgressNotificationParams, ProgressTokenValue,
    PromptListChangedNotification, ResourceListChangedNotification, ResourceUpdatedNotification,
    ResourceUpdatedNotificationParams, RootsListChangedNotification, TaskStatusNotification,
    ToolListChangedNotification,
};
pub use ping::{EmptyParams, EmptyResult, PingRequest};
pub use schema::JsonSchema;
pub use traits::{
    HasData, HasDataParam, HasMeta, HasMetaParam, HasProgressTokenParam, JsonRpcNotificationTrait,
    JsonRpcRequestTrait, JsonRpcResponseTrait, Params, RpcResult,
};

// JSON-RPC foundation (legacy - prefer our implementations above)
pub use turul_mcp_json_rpc_server::{
    RequestParams as LegacyRequestParams, ResponseResult, types::RequestId,
};

/// The MCP protocol version implemented by this crate
pub const MCP_VERSION: &str = "2025-11-25";

/// Common result type for MCP operations
pub type McpResult<T> = Result<T, McpError>;

/// MCP-specific errors
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("Protocol version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: String, actual: String },

    #[error("Invalid capability: {0}")]
    InvalidCapability(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    #[error("Prompt not found: {0}")]
    PromptNotFound(String),

    #[error("Invalid request: {message}")]
    InvalidRequest { message: String },

    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    #[error("Missing required parameter: {0}")]
    MissingParameter(String),

    #[error("Invalid parameter type for '{param}': expected {expected}, got {actual}")]
    InvalidParameterType {
        param: String,
        expected: String,
        actual: String,
    },

    #[error("Parameter '{param}' value {value} is out of range: {constraint}")]
    ParameterOutOfRange {
        param: String,
        value: String,
        constraint: String,
    },

    #[error("Tool execution failed: {0}")]
    ToolExecutionError(String),

    #[error("Resource execution failed: {0}")]
    ResourceExecutionError(String),

    #[error("Prompt execution failed: {0}")]
    PromptExecutionError(String),

    #[error("Resource access denied: {0}")]
    ResourceAccessDenied(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Session error: {0}")]
    SessionError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Transport error: {0}")]
    TransportError(String),

    #[error("JSON-RPC protocol error: {0}")]
    JsonRpcProtocolError(String),

    /// A JSON-RPC error with preserved code, message, and optional data.
    ///
    /// Used by `tasks/result` to reproduce the original error verbatim, as
    /// required by the MCP spec: "tasks/result MUST return that same JSON-RPC error."
    #[error("JSON-RPC error {code}: {message}")]
    JsonRpcError {
        code: i64,
        message: String,
        data: Option<serde_json::Value>,
    },
}

impl From<String> for McpError {
    fn from(message: String) -> Self {
        Self::ToolExecutionError(message)
    }
}

impl From<&str> for McpError {
    fn from(message: &str) -> Self {
        Self::ToolExecutionError(message.to_string())
    }
}

impl McpError {
    /// Create a missing parameter error
    pub fn missing_param(param: &str) -> Self {
        Self::MissingParameter(param.to_string())
    }

    /// Create an invalid parameter type error
    pub fn invalid_param_type(param: &str, expected: &str, actual: &str) -> Self {
        Self::InvalidParameterType {
            param: param.to_string(),
            expected: expected.to_string(),
            actual: actual.to_string(),
        }
    }

    /// Create a parameter out of range error
    pub fn param_out_of_range(param: &str, value: &str, constraint: &str) -> Self {
        Self::ParameterOutOfRange {
            param: param.to_string(),
            value: value.to_string(),
            constraint: constraint.to_string(),
        }
    }

    /// Create a tool execution error
    pub fn tool_execution(message: &str) -> Self {
        Self::ToolExecutionError(message.to_string())
    }

    /// Create a resource execution error
    pub fn resource_execution(message: &str) -> Self {
        Self::ResourceExecutionError(message.to_string())
    }

    /// Create a prompt execution error
    pub fn prompt_execution(message: &str) -> Self {
        Self::PromptExecutionError(message.to_string())
    }

    /// Create a validation error
    pub fn validation(message: &str) -> Self {
        Self::ValidationError(message.to_string())
    }

    /// Create a configuration error
    pub fn configuration(message: &str) -> Self {
        Self::ConfigurationError(message.to_string())
    }

    /// Create a transport error
    pub fn transport(message: &str) -> Self {
        Self::TransportError(message.to_string())
    }

    /// Create a JSON-RPC protocol error
    pub fn json_rpc_protocol(message: &str) -> Self {
        Self::JsonRpcProtocolError(message.to_string())
    }

    /// Create a JSON-RPC error with preserved code, message, and optional data.
    ///
    /// Used by `tasks/result` to reproduce original errors verbatim.
    pub fn json_rpc_error(
        code: i64,
        message: impl Into<String>,
        data: Option<serde_json::Value>,
    ) -> Self {
        Self::JsonRpcError {
            code,
            message: message.into(),
            data,
        }
    }

    /// Convert to a JsonRpcErrorObject for JSON-RPC 2.0 responses
    pub fn to_error_object(&self) -> turul_mcp_json_rpc_server::error::JsonRpcErrorObject {
        use turul_mcp_json_rpc_server::error::JsonRpcErrorObject;

        match self {
            // Request-level errors map to InvalidParams (-32602) with descriptive message
            McpError::InvalidRequest { message } => JsonRpcErrorObject::invalid_params(message),

            // Parameter-related errors map to InvalidParams (-32602)
            McpError::InvalidParameters(msg) => JsonRpcErrorObject::invalid_params(msg),
            McpError::MissingParameter(param) => JsonRpcErrorObject::invalid_params(&format!(
                "Missing required parameter: {}",
                param
            )),
            McpError::InvalidParameterType {
                param,
                expected,
                actual,
            } => JsonRpcErrorObject::invalid_params(&format!(
                "Invalid parameter type for '{}': expected {}, got {}",
                param, expected, actual
            )),
            McpError::ParameterOutOfRange {
                param,
                value,
                constraint,
            } => JsonRpcErrorObject::invalid_params(&format!(
                "Parameter '{}' value {} is out of range: {}",
                param, value, constraint
            )),

            // Not found errors map to server errors
            McpError::ToolNotFound(name) => {
                JsonRpcErrorObject::server_error(-32001, &format!("Tool not found: {}", name), None)
            }
            McpError::ResourceNotFound(uri) => JsonRpcErrorObject::server_error(
                -32002,
                &format!("Resource not found: {}", uri),
                None,
            ),
            McpError::PromptNotFound(name) => JsonRpcErrorObject::server_error(
                -32003,
                &format!("Prompt not found: {}", name),
                None,
            ),

            // Access and execution errors
            McpError::ToolExecutionError(msg) => JsonRpcErrorObject::server_error(
                -32010,
                &format!("Tool execution failed: {}", msg),
                None,
            ),
            McpError::ResourceExecutionError(msg) => JsonRpcErrorObject::server_error(
                -32012,
                &format!("Resource execution failed: {}", msg),
                None,
            ),
            McpError::PromptExecutionError(msg) => JsonRpcErrorObject::server_error(
                -32013,
                &format!("Prompt execution failed: {}", msg),
                None,
            ),
            McpError::ResourceAccessDenied(uri) => JsonRpcErrorObject::server_error(
                -32011,
                &format!("Resource access denied: {}", uri),
                None,
            ),

            // Validation errors
            McpError::ValidationError(msg) => JsonRpcErrorObject::server_error(
                -32020,
                &format!("Validation error: {}", msg),
                None,
            ),
            McpError::InvalidCapability(cap) => JsonRpcErrorObject::server_error(
                -32021,
                &format!("Invalid capability: {}", cap),
                None,
            ),
            McpError::VersionMismatch { expected, actual } => JsonRpcErrorObject::server_error(
                -32022,
                &format!(
                    "Protocol version mismatch: expected {}, got {}",
                    expected, actual
                ),
                None,
            ),

            // Configuration and session errors
            McpError::ConfigurationError(msg) => JsonRpcErrorObject::server_error(
                -32030,
                &format!("Configuration error: {}", msg),
                None,
            ),
            McpError::SessionError(msg) => {
                JsonRpcErrorObject::server_error(-32031, &format!("Session error: {}", msg), None)
            }

            // Transport and protocol layer errors
            McpError::TransportError(msg) => {
                JsonRpcErrorObject::server_error(-32040, &format!("Transport error: {}", msg), None)
            }
            McpError::JsonRpcProtocolError(msg) => JsonRpcErrorObject::server_error(
                -32041,
                &format!("JSON-RPC protocol error: {}", msg),
                None,
            ),

            // I/O and serialization errors map to internal errors
            McpError::IoError(err) => {
                JsonRpcErrorObject::internal_error(Some(format!("IO error: {}", err)))
            }
            McpError::SerializationError(err) => {
                JsonRpcErrorObject::internal_error(Some(format!("Serialization error: {}", err)))
            }

            // Pass-through: preserves original code/message/data verbatim
            McpError::JsonRpcError {
                code,
                message,
                data,
            } => JsonRpcErrorObject::server_error(*code, message, data.clone()),
        }
    }

    /// Create a JSON-RPC error response for this MCP error
    pub fn to_json_rpc_response(
        &self,
        id: Option<turul_mcp_json_rpc_server::RequestId>,
    ) -> turul_mcp_json_rpc_server::JsonRpcError {
        turul_mcp_json_rpc_server::JsonRpcError::new(id, self.to_error_object())
    }

    /// Legacy method for backward compatibility - use to_error_object() instead
    #[deprecated(note = "Use to_error_object() instead for cleaner architecture")]
    pub fn to_json_rpc_error(&self) -> turul_mcp_json_rpc_server::error::JsonRpcErrorObject {
        self.to_error_object()
    }
}

// Implement the ToJsonRpcError trait for MCP errors
impl turul_mcp_json_rpc_server::r#async::ToJsonRpcError for McpError {
    fn to_error_object(&self) -> turul_mcp_json_rpc_server::error::JsonRpcErrorObject {
        // Delegate to our existing type-safe implementation
        McpError::to_error_object(self)
    }
}
