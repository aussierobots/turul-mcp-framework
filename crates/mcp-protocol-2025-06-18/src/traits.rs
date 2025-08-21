//! Traits for JSON-RPC types as per MCP specification (2025-06-18)
//!
//! These traits provide a structured approach to handling MCP protocol messages
//! and responses, ensuring compliance with the latest specification.

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

use crate::meta::ProgressToken;

// ====================
// === Base Traits ====
// ====================

/// Marker trait for parameter types
pub trait Params {}

/// Trait for types that have a JSON-RPC request ID
pub trait HasRequestId {
    fn id(&self) -> &str;
}

/// Trait for types that have a result field
pub trait HasResult {
    fn result(&self) -> &dyn RpcResult;
}

/// Trait for types that have a JSON-RPC version
pub trait HasJsonRpcVersion {
    fn version(&self) -> &str {
        "2.0"
    }
}

/// Trait for types that have a method name
pub trait HasMethod {
    fn method(&self) -> &str;
}

/// Trait for types that have parameters
pub trait HasParams {
    fn params(&self) -> Option<&dyn Params>;
}

/// Trait for types that have structured data
pub trait HasData {
    /// Returns an owned JSON object map of this value
    fn data(&self) -> HashMap<String, Value>;
}

/// Trait for types that have _meta information
pub trait HasMeta {
    /// Returns an owned JSON object map of _meta fields
    fn meta(&self) -> Option<HashMap<String, Value>>;
}

/// Trait for types that have error objects
pub trait HasErrorObject {
    fn error(&self) -> Option<&serde_json::Value>;
}

// ========================
// === Derived Traits =====
// ========================

/// JSON-RPC request trait (combines method + params)
pub trait RpcRequest: HasMethod + HasParams {}

/// JSON-RPC notification trait (combines method + params)
pub trait RpcNotification: HasMethod + HasParams {}

/// RPC result trait (combines data + meta)
pub trait RpcResult: HasMeta + HasData {}

/// Complete JSON-RPC request trait
pub trait JsonRpcRequestTrait: HasJsonRpcVersion + HasRequestId + RpcRequest {}

/// Complete JSON-RPC notification trait  
pub trait JsonRpcNotificationTrait: HasJsonRpcVersion + RpcNotification {}

/// Complete JSON-RPC response trait
pub trait JsonRpcResponseTrait: HasJsonRpcVersion + HasRequestId + HasResult + Serialize {
    fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }
}

// ========================
// === Parameter Traits ===
// ========================

/// Trait for parameters that include a progress token
pub trait HasProgressTokenParam: Params {
    fn progress_token(&self) -> Option<&ProgressToken>;
}

/// Trait for parameters that include a cursor
pub trait HasCursorParam: Params {
    fn cursor(&self) -> Option<&str>;
}

/// Trait for parameters that include structured data
pub trait HasDataParam: Params {
    fn data(&self) -> &HashMap<String, Value>;
}

/// Trait for parameters that include _meta information
pub trait HasMetaParam: Params {
    fn meta(&self) -> Option<&HashMap<String, Value>>;
}

/// Trait for parameters that include a reason field
pub trait HasReasonParam: Params {
    fn reason(&self) -> Option<&str>;
}

// ========================
// === Specific Traits ===
// ========================

/// Traits for specific MCP endpoints - these align with GPS Trust implementation

pub trait InitializeResult: RpcResult {}
pub trait ListToolsResult: RpcResult {}
pub trait CallToolResult: RpcResult {}
pub trait ListResourcesResult: RpcResult {}
pub trait ReadResourceResult: RpcResult {}
pub trait ListPromptsResult: RpcResult {}
pub trait GetPromptResult: RpcResult {}
pub trait ListRootsResult: RpcResult {}
pub trait CreateMessageResult: RpcResult {}
pub trait ListResourceTemplatesResult: RpcResult {}

// Parameter traits for specific endpoints
pub trait HasInitializeParams: Params {
    fn client_info(&self) -> Option<&HashMap<String, Value>>;
    fn capabilities(&self) -> Option<&HashMap<String, Value>>;
}

pub trait HasListToolsParams: Params {}

pub trait HasCallToolParams: Params {
    fn name(&self) -> &str;
    fn arguments(&self) -> Option<&HashMap<String, Value>>;
}

pub trait HasListResourcesParams: Params {}

pub trait HasReadResourceParams: Params {
    fn uri(&self) -> &str;
}

pub trait HasListPromptsParams: Params {}

pub trait HasGetPromptParams: Params {
    fn name(&self) -> &str;
    fn arguments(&self) -> Option<&HashMap<String, Value>>;
}

pub trait HasListRootsParams: Params {}

pub trait HasCreateMessageParams: Params {
    fn messages(&self) -> &[Value];
    fn model_preferences(&self) -> Option<&HashMap<String, Value>>;
}

pub trait HasListResourceTemplatesParams: Params {}

// ========================
// === Request Traits =====
// ========================

pub trait InitializeRequest: JsonRpcRequestTrait + HasInitializeParams {}
pub trait ListToolsRequest: JsonRpcRequestTrait + HasListToolsParams {}
pub trait CallToolRequest: JsonRpcRequestTrait + HasCallToolParams {}
pub trait ListResourcesRequest: JsonRpcRequestTrait + HasListResourcesParams {}
pub trait ReadResourceRequest: JsonRpcRequestTrait + HasReadResourceParams {}
pub trait ListPromptsRequest: JsonRpcRequestTrait + HasListPromptsParams {}
pub trait GetPromptRequest: JsonRpcRequestTrait + HasGetPromptParams {}
pub trait ListRootsRequest: JsonRpcRequestTrait + HasListRootsParams {}
pub trait CreateMessageRequest: JsonRpcRequestTrait + HasCreateMessageParams {}
pub trait ListResourceTemplatesRequest: JsonRpcRequestTrait + HasListResourceTemplatesParams {}