//! Traits for JSON-RPC types as per MCP specification (2025-06-18)

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

use crate::{
    completion::CompleteArgument,
    initialize::{ClientCapabilities, Implementation, ServerCapabilities},
    logging::LogLevel,
    meta::{Cursor, ProgressToken},
    prompts::{Prompt, PromptMessage},
    resources::Resource,
    roots::Root,
    sampling::{MessageContent, SamplingMessage},
    tools::{Tool, ToolResult},
    version::McpVersion,
};

// JSON-RPC version constant
pub const JSONRPC_VERSION: &str = "2.0";

// ====================
// === Base Traits ====
// ====================

pub trait Params {}

pub trait HasRequestId {
    fn id(&self) -> &mcp_json_rpc_server::types::RequestId;
}

pub trait HasResult {
    fn result(&self) -> &dyn RpcResult;
}

pub trait HasJsonRpcVersion {
    fn version(&self) -> &str {
        JSONRPC_VERSION
    }
}

pub trait HasMethod {
    fn method(&self) -> &str;
}

pub trait HasParams {
    fn params(&self) -> Option<&dyn Params>;
}

pub trait HasData {
    /// Returns an owned JSON‐object map of this value.
    fn data(&self) -> HashMap<String, Value>;
}

pub trait HasMeta {
    /// Returns an owned JSON‐object map of this value.
    fn meta(&self) -> Option<HashMap<String, Value>>;
}

pub trait HasErrorObject {
    fn error(&self) -> &mcp_json_rpc_server::error::JsonRpcErrorObject;
}

// ==========================
// === Derived Interfaces ===
// ==========================

pub trait RpcRequest: HasMethod + HasParams {}
pub trait RpcNotification: HasMethod + HasParams {}
pub trait RpcResult: HasMeta + HasData {}
pub trait JsonRpcRequestTrait: HasJsonRpcVersion + HasRequestId + RpcRequest {}
pub trait JsonRpcNotificationTrait: HasJsonRpcVersion + RpcNotification {}

pub trait JsonRpcResponseTrait: HasJsonRpcVersion + HasRequestId + HasResult + Serialize {
    fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }
}

pub trait JsonRpcErrorTrait: HasJsonRpcVersion + HasRequestId + HasErrorObject {}

// ==========================
// === Param Specialisations ===
// ==========================

pub trait HasRequestIdParam: Params {
    fn request_id(&self) -> &mcp_json_rpc_server::types::RequestId;
}

pub trait HasReasonParam: Params {
    fn reason(&self) -> Option<&str>;
}

pub trait HasDataParam: Params {
    fn data(&self) -> &HashMap<String, Value>;
}

pub trait HasMetaParam: Params {
    fn meta(&self) -> Option<&HashMap<String, Value>>;
}

pub trait HasProgressTokenParam: Params {
    fn progress_token(&self) -> Option<&ProgressToken>;
}

pub trait HasInitializeParams: Params {
    fn protocol_version(&self) -> McpVersion;
    fn capabilities(&self) -> &ClientCapabilities;
    fn client_info(&self) -> &Implementation;
}

// ==========================
// === Typed Traits from MCP Spec ===
// ==========================

pub trait HasCancelledParams: HasRequestIdParam + HasReasonParam {}
pub trait CancelledNotification: RpcNotification + HasCancelledParams {}

pub trait InitializeRequest: JsonRpcRequestTrait + HasInitializeParams {
    fn method(&self) -> &str {
        "initialize"
    }
}

pub trait InitializeResult: RpcResult {
    fn protocol_version(&self) -> &str;
    fn capabilities(&self) -> &ServerCapabilities;
    fn server_info(&self) -> &Implementation;
    fn instructions(&self) -> Option<&str>;
}

pub trait InitializeNotification: JsonRpcNotificationTrait {
    fn method(&self) -> &str {
        "notifications/initialized"
    }
}

// ---------------------- notifications/progress ------------------------

/// Trait for params of `notifications/progress`
pub trait HasProgressParams: Params {
    fn progress_token(&self) -> &ProgressToken;
    fn progress(&self) -> u64;
    fn total(&self) -> Option<u64>;
    fn message(&self) -> Option<&String>;
}

/// The notification itself
pub trait ProgressNotification: JsonRpcNotificationTrait + HasProgressParams {
    /// Always exactly `"notifications/progress"`
    fn method(&self) -> &str {
        "notifications/progress"
    }
}

// ---------------------- resources/list ------------------------

pub trait HasListResourcesParams: Params {
    fn cursor(&self) -> Option<&Cursor>;
}

pub trait ListResourcesRequest: JsonRpcRequestTrait + HasListResourcesParams {
    fn method(&self) -> &str {
        "resources/list"
    }
}

pub trait ListResourcesResult: RpcResult {
    fn resources(&self) -> &Vec<Resource>;
    fn next_cursor(&self) -> Option<&Cursor>;
}

pub trait ResourcesListChangedNotification: JsonRpcNotificationTrait {
    fn method(&self) -> &str {
        "notifications/resources/listChanged"
    }
}

pub trait HasReadResourceParams: Params {
    fn uri(&self) -> &String;
}

pub trait ReadResourceRequest: JsonRpcRequestTrait + HasReadResourceParams {
    fn method(&self) -> &str {
        "resources/read"
    }
}

pub trait ReadResourceResult: RpcResult {
    fn contents(&self) -> &Vec<crate::resources::ResourceContent>;
}

pub trait HasResourceUpdatedParams: Params {
    fn uri(&self) -> &String;
}

pub trait ResourceUpdatedNotification: JsonRpcNotificationTrait + HasResourceUpdatedParams {
    fn method(&self) -> &str {
        "notifications/resources/updated"
    }
}

// ---------------------- prompts/list & get ------------------------

pub trait HasListPromptsParams: Params {
    fn cursor(&self) -> Option<&Cursor>;
}

pub trait ListPromptsRequest: JsonRpcRequestTrait + HasListPromptsParams {
    fn method(&self) -> &str {
        "prompts/list"
    }
}

pub trait ListPromptsResult: RpcResult {
    fn prompts(&self) -> &Vec<Prompt>;
    fn next_cursor(&self) -> Option<&Cursor>;
}

pub trait HasGetPromptParams: Params {
    fn name(&self) -> &String;
    fn arguments(&self) -> Option<&HashMap<String, Value>>;
}

pub trait GetPromptRequest: JsonRpcRequestTrait + HasGetPromptParams {
    fn method(&self) -> &str {
        "prompts/get"
    }
}

pub trait GetPromptResult: RpcResult {
    fn description(&self) -> Option<&String>;
    fn messages(&self) -> &Vec<PromptMessage>;
}

pub trait PromptListChangedNotification: JsonRpcNotificationTrait {
    fn method(&self) -> &str {
        "notifications/prompts/listChanged"
    }
}

// ---------------------- tools/list & call ------------------------

pub trait HasListToolsParams: Params {
    fn cursor(&self) -> Option<&Cursor>;
}

pub trait ListToolsRequest: JsonRpcRequestTrait + HasListToolsParams {
    fn method(&self) -> &str {
        "tools/list"
    }
}

pub trait ListToolsResult: RpcResult {
    fn tools(&self) -> &Vec<Tool>;
    fn next_cursor(&self) -> Option<&Cursor>;
}

pub trait HasCallToolParams: Params {
    fn name(&self) -> &String;
    fn arguments(&self) -> Option<&Value>;
    fn meta(&self) -> Option<&HashMap<String, Value>>;
}

pub trait CallToolRequest: JsonRpcRequestTrait + HasCallToolParams {
    fn method(&self) -> &str {
        "tools/call"
    }
}

pub trait CallToolResult: RpcResult {
    fn content(&self) -> &Vec<ToolResult>;
    fn is_error(&self) -> Option<bool>;
    /// Structured content that matches the tool's output schema (MCP 2025-06-18)
    fn structured_content(&self) -> Option<&Value>;
}

pub trait ToolListChangedNotification: JsonRpcNotificationTrait {
    fn method(&self) -> &str {
        "notifications/tools/listChanged"
    }
}

// ---------------------- sampling/createMessage ------------------------

pub trait HasCreateMessageParams: Params {
    fn messages(&self) -> &Vec<SamplingMessage>;
    fn model_preferences(&self) -> Option<&Value>;
    fn system_prompt(&self) -> Option<&String>;
    fn include_context(&self) -> Option<&String>;
    fn temperature(&self) -> Option<&f64>;
    fn max_tokens(&self) -> u32;
    fn stop_sequences(&self) -> Option<&Vec<String>>;
    fn metadata(&self) -> Option<&Value>;
}

pub trait CreateMessageRequest: JsonRpcRequestTrait + HasCreateMessageParams {
    fn method(&self) -> &str {
        "sampling/createMessage"
    }
}

pub trait CreateMessageResult: RpcResult {
    fn role(&self) -> &str;
    fn content(&self) -> &MessageContent;
    fn model(&self) -> &String;
    fn stop_reason(&self) -> Option<&String>;
}

// ---------------------- completion/complete ------------------------

/// The `params` object for `completion/complete`
pub trait HasCompleteParams: Params {
    /// The prompt or resource reference to complete against.
    fn reference(&self) -> &Value;
    /// The name/value pair to complete.
    fn argument(&self) -> &CompleteArgument;
    /// Optional additional context.
    fn context(&self) -> Option<&Value>;
}

/// The JSON-RPC request for `completion/complete`
pub trait CompleteRequestTrait: JsonRpcRequestTrait + HasCompleteParams {
    /// Always exactly `"completion/complete"`
    fn method(&self) -> &str {
        "completion/complete"
    }
}

/// Exposes the inner `completion` field of the response payload.
pub trait HasCompletionResult: RpcResult {
    fn completion(&self) -> &Value;
}

/// The JSON-RPC result for `completion/complete`
pub trait CompleteResult: RpcResult + HasCompletionResult {}

// ---------------------- templates/list ------------------------

pub trait HasListResourceTemplatesParams: Params {
    fn cursor(&self) -> Option<&Cursor>;
}

pub trait ListResourceTemplatesRequest:
    JsonRpcRequestTrait + HasListResourceTemplatesParams
{
    fn method(&self) -> &str {
        "resources/templates/list"
    }
}

pub trait ListResourceTemplatesResult: RpcResult {
    fn resource_templates(&self) -> &Vec<Value>;
    fn next_cursor(&self) -> Option<&Cursor>;
}

// ---------------------- roots/list ------------------------

pub trait HasListRootsParams: Params {}

pub trait ListRootsRequest: JsonRpcRequestTrait + HasListRootsParams {
    fn method(&self) -> &str {
        "roots/list"
    }
}

pub trait ListRootsResult: RpcResult {
    fn roots(&self) -> &Vec<Root>;
}

pub trait RootsListChangedNotification: JsonRpcNotificationTrait {
    fn method(&self) -> &str {
        "notifications/roots/listChanged"
    }
}

// ---------------------- logging ------------------------

pub trait HasSetLevelParams: Params {
    fn level(&self) -> &LogLevel;
}

pub trait SetLevelRequest: JsonRpcRequestTrait + HasSetLevelParams {
    fn method(&self) -> &str {
        "logging/setLevel"
    }
}

// Field-getter traits you already have; if not, add:
pub trait HasLevelParam: Params {
    fn level(&self) -> &LogLevel;
}
pub trait HasLoggerParam: Params {
    fn logger(&self) -> Option<&String>;
}

pub trait LoggingMessageNotificationTrait: JsonRpcNotificationTrait + HasParams {
    fn method(&self) -> &str {
        "notifications/message"
    }
}

// ---------------------- elicitation ------------------------

pub trait HasElicitParams: Params {
    fn message(&self) -> &String;
    fn requested_schema(&self) -> &Value;
}

pub trait ElicitRequest: JsonRpcRequestTrait + HasElicitParams {
    fn method(&self) -> &str {
        "elicitation/create"
    }
}

pub trait ElicitResult: RpcResult {
    fn action(&self) -> &Value;
    fn content(&self) -> Option<&HashMap<String, Value>>;
}

// ---------------------- trait-based parameter extraction ------------------------

/// Trait for extracting parameters from RequestParams using trait constraints
pub trait ParamExtractor<T: Params> {
    type Error;

    /// Extract parameters from RequestParams using trait-based conversion
    fn extract(params: mcp_json_rpc_server::RequestParams) -> Result<T, Self::Error>;
}

/// Trait for serde-based parameter extraction (simpler cases)
pub trait SerdeParamExtractor<T: Params> {
    type Error;

    /// Extract parameters using serde deserialization
    fn extract_serde(params: mcp_json_rpc_server::RequestParams) -> Result<T, Self::Error>;
}

/// Trait for field-by-field parameter extraction (complex cases)
pub trait FieldParamExtractor<T: Params> {
    type Error;

    /// Extract parameters field by field with validation
    fn extract_fields(params: mcp_json_rpc_server::RequestParams) -> Result<T, Self::Error>;
}
