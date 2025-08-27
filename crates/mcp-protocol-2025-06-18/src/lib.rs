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
pub mod elicitation;
pub mod notifications;
pub mod ping;
pub mod schema;
pub mod meta;
pub mod traits;
pub mod json_rpc;
pub mod param_extraction;

#[cfg(test)]
mod compliance_test;

// Re-export main types
pub use version::McpVersion;
pub use initialize::{
    InitializeRequest, InitializeResult, 
    ClientCapabilities, ServerCapabilities, Implementation
};
pub use tools::{
    Tool, ToolResult, ToolSchema,
    ListToolsRequest, ListToolsResult,
    CallToolRequest, CallToolResult
};
pub use resources::{
    Resource, ResourceContent, ListResourcesRequest, ListResourcesResult,
    ReadResourceRequest, ReadResourceResult, ResourceSubscription,
    SubscribeRequest, UnsubscribeRequest
};
pub use prompts::{
    Prompt, PromptMessage, PromptArgument,
    GetPromptRequest, GetPromptResult,
    ListPromptsRequest, ListPromptsResult
};
// ResourceTemplate functionality is now part of resources module
// pub use resources::{ResourceTemplate, ListResourceTemplatesRequest, ListResourceTemplatesResult};
pub use elicitation::{
    ElicitCreateRequest, ElicitCreateParams, ElicitResult, ElicitAction,
    PrimitiveSchemaDefinition, ElicitationSchema, StringFormat, ElicitationBuilder
};
pub use ping::{
    PingRequest, EmptyResult, EmptyParams
};
pub use notifications::{
    CancelledNotification, InitializedNotification, ProgressNotification, ProgressNotificationParams,
    LoggingMessageNotification, LoggingMessageNotificationParams,
    ResourceListChangedNotification, ResourceUpdatedNotification, ResourceUpdatedNotificationParams,
    PromptListChangedNotification, ToolListChangedNotification, RootsListChangedNotification,
    NotificationParams, Notification
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
pub use mcp_json_rpc_server::{
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
    JsonRpcError(#[from] mcp_json_rpc_server::JsonRpcError),
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
    
    /// Convert to a JsonRpcErrorObject for JSON-RPC 2.0 responses
    pub fn to_json_rpc_error(&self) -> mcp_json_rpc_server::error::JsonRpcErrorObject {
        use mcp_json_rpc_server::error::JsonRpcErrorObject;
        
        match self {
            // Parameter-related errors map to InvalidParams (-32602)
            McpError::InvalidParameters(msg) => 
                JsonRpcErrorObject::invalid_params(msg),
            McpError::MissingParameter(param) => 
                JsonRpcErrorObject::invalid_params(&format!("Missing required parameter: {}", param)),
            McpError::InvalidParameterType { param, expected, actual } => 
                JsonRpcErrorObject::invalid_params(&format!(
                    "Invalid parameter type for '{}': expected {}, got {}", param, expected, actual)),
            McpError::ParameterOutOfRange { param, value, constraint } => 
                JsonRpcErrorObject::invalid_params(&format!(
                    "Parameter '{}' value {} is out of range: {}", param, value, constraint)),
                    
            // Not found errors map to server errors
            McpError::ToolNotFound(name) => 
                JsonRpcErrorObject::server_error(-32001, &format!("Tool not found: {}", name), None),
            McpError::ResourceNotFound(uri) => 
                JsonRpcErrorObject::server_error(-32002, &format!("Resource not found: {}", uri), None),
            McpError::PromptNotFound(name) => 
                JsonRpcErrorObject::server_error(-32003, &format!("Prompt not found: {}", name), None),
                
            // Access and execution errors
            McpError::ToolExecutionError(msg) => 
                JsonRpcErrorObject::server_error(-32010, &format!("Tool execution failed: {}", msg), None),
            McpError::ResourceAccessDenied(uri) => 
                JsonRpcErrorObject::server_error(-32011, &format!("Resource access denied: {}", uri), None),
                
            // Validation errors
            McpError::ValidationError(msg) => 
                JsonRpcErrorObject::server_error(-32020, &format!("Validation error: {}", msg), None),
            McpError::InvalidCapability(cap) => 
                JsonRpcErrorObject::server_error(-32021, &format!("Invalid capability: {}", cap), None),
            McpError::VersionMismatch { expected, actual } => 
                JsonRpcErrorObject::server_error(-32022, &format!(
                    "Protocol version mismatch: expected {}, got {}", expected, actual), None),
                    
            // Configuration and session errors
            McpError::ConfigurationError(msg) => 
                JsonRpcErrorObject::server_error(-32030, &format!("Configuration error: {}", msg), None),
            McpError::SessionError(msg) => 
                JsonRpcErrorObject::server_error(-32031, &format!("Session error: {}", msg), None),
                
            // I/O and serialization errors map to internal errors
            McpError::IoError(err) => 
                JsonRpcErrorObject::internal_error(Some(format!("IO error: {}", err))),
            McpError::SerializationError(err) => 
                JsonRpcErrorObject::internal_error(Some(format!("Serialization error: {}", err))),
                
            // Nested JSON-RPC errors are passed through
            McpError::JsonRpcError(err) => err.error.clone(),
        }
    }
    
    /// Create a JSON-RPC error response for this MCP error
    pub fn to_json_rpc_response(&self, id: Option<mcp_json_rpc_server::RequestId>) -> mcp_json_rpc_server::JsonRpcError {
        mcp_json_rpc_server::JsonRpcError::new(id, self.to_json_rpc_error())
    }
}