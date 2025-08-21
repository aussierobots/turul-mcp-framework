//! # Model Context Protocol (MCP) - 2025-06-18 Specification
//!
//! This crate provides a complete implementation of the Model Context Protocol (MCP)
//! specification version 2025-06-18. It includes all the types, traits, and utilities
//! needed to build MCP-compliant servers and clients.
//!
//! ## Features
//! - Complete MCP 2025-06-18 specification compliance
//! - Support for all MCP capabilities (tools, resources, prompts, etc.)
//! - Built on top of the json-rpc-server foundation
//! - Support for streamable HTTP and _meta fields
//! - Progress tokens and cursor support
//! - Structured user elicitation via JSON Schema

pub mod version;
pub mod initialize;
pub mod tools;
pub mod resources;
pub mod prompts;
pub mod completion;
pub mod logging;
pub mod roots;
pub mod sampling;
pub mod templates;
pub mod elicitation;
pub mod notifications;
pub mod schema;
pub mod meta;
pub mod traits;
pub mod json_rpc;

// Re-export main types
pub use version::McpVersion;
pub use initialize::{
    InitializeRequest, InitializeResponse, 
    ClientCapabilities, ServerCapabilities, Implementation
};
pub use tools::{
    Tool, ToolResult, ToolSchema, Cursor,
    ListToolsRequest, ListToolsResponse,
    CallToolRequest, CallToolResponse
};
pub use resources::{
    Resource, ResourceContent, ListResourcesRequest, ListResourcesResponse,
    ReadResourceRequest, ReadResourceResponse, ResourceSubscription
};
pub use prompts::{
    Prompt, PromptMessage, PromptArgument,
    GetPromptRequest, GetPromptResponse,
    ListPromptsRequest, ListPromptsResponse
};
pub use elicitation::{
    ElicitationRequest, ElicitationResponse, ElicitationRequestParams,
    ElicitationRequestResult, ElicitationCancelledNotification, ElicitationBuilder
};
pub use notifications::{
    CancelledNotification, InitializedNotification, ProgressNotificationParams,
    LoggingMessageNotification, ResourceListChangedNotificationParams,
    ResourceUpdatedNotificationParams, PromptListChangedNotificationParams,
    ToolListChangedNotificationParams, RootListChangedNotificationParams
};
pub use schema::JsonSchema;
pub use meta::{Meta, ProgressToken, Cursor as MetaCursor, WithMeta, PaginatedResponse, ProgressResponse};
pub use traits::{
    RpcResult, HasData, HasMeta, HasProgressTokenParam, HasDataParam, HasMetaParam,
    Params, JsonRpcRequestTrait, JsonRpcNotificationTrait, JsonRpcResponseTrait
};
pub use json_rpc::{
    JsonRpcRequest, JsonRpcResponse, JsonRpcNotification, JsonRpcMessage,
    RequestParams, ResultWithMeta, JsonRpcError
};

// JSON-RPC foundation (legacy - prefer our implementations above)
pub use json_rpc_server::{
    RequestParams as LegacyRequestParams, 
    ResponseResult, 
    types::RequestId
};

/// The MCP protocol version implemented by this crate
pub const MCP_VERSION: &str = "2025-06-18";

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
    
    #[error("JSON-RPC error: {0}")]
    JsonRpcError(#[from] json_rpc_server::JsonRpcError),
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
    
    /// Create a validation error
    pub fn validation(message: &str) -> Self {
        Self::ValidationError(message.to_string())
    }
    
    /// Create a configuration error
    pub fn configuration(message: &str) -> Self {
        Self::ConfigurationError(message.to_string())
    }
}