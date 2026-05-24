//! Framework traits for JSON-RPC types per the MCP DRAFT-2026-v1 specification.
//!
//! Trait names follow the schema's TypeScript interface names (`CallToolRequest`,
//! `CallToolResult`, etc.). The `Has*Params` helpers expose per-interface field
//! access without leaking the concrete Rust struct, so framework dispatchers
//! can work with any Params implementation.

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

use crate::{
    completion::CompleteArgument,
    logging::LogLevel,
    meta::{Cursor, ProgressToken},
    prompts::{Prompt, PromptMessage},
    resources::Resource,
    roots::Root,
    sampling::{ModelPreferences, Role, SamplingMessage, SamplingMessageContent},
    tools::{Tool, ToolResult},
};

// JSON-RPC version constant
pub const JSONRPC_VERSION: &str = "2.0";

// ====================
// === Base Traits ====
// ====================

pub trait Params {}

/// Required request id — present on requests and successful responses.
/// Schema: `RequestId = string | number`; we expose as borrowed `Value` for
/// permissive accept (the typed narrowing is a separate follow-up).
pub trait HasRequestId {
    fn id(&self) -> &Value;
}

/// Optional request id — used by `JSONRPCErrorResponse` where the server may
/// omit `id` if it couldn't parse the original request's id.
pub trait HasOptionalRequestId {
    fn id(&self) -> Option<&Value>;
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

/// Exposes the `_meta` field per the schema's `Result { _meta?: MetaObject }`
/// and `Notification.params { _meta?: MetaObject }` shapes. Borrowed, typed —
/// no JSON round-trip.
pub trait HasMeta {
    fn meta(&self) -> Option<&crate::meta::MetaObject>;
}

/// Exposes the `resultType` discriminator per the schema's
/// `Result.resultType: ResultType` field. Every spec-compliant result MUST
/// carry this; per the schema doc-comment, absent values default to `Complete`
/// for backward compatibility with pre-DRAFT-2026-v1 servers.
pub trait HasResultType {
    fn result_type(&self) -> crate::result_type::ResultType;
}

/// Escape hatch — flatten a typed result into a JSON-object map. Not part of
/// the [`RpcResult`] supertrait bound; consumers that need this should prefer
/// `serde_json::to_value`. Retained for the rare framework site that needs an
/// untyped view.
pub trait HasData {
    fn data(&self) -> HashMap<String, Value>;
}

/// Exposes the `error` field on a `JSONRPCErrorResponse`. The schema's
/// `Error` interface maps to [`turul_rpc::error::JsonRpcErrorObject`]
/// (`{ code, message, data? }`).
pub trait HasErrorObject {
    fn error(&self) -> &turul_rpc::error::JsonRpcErrorObject;
}

// ==========================
// === Derived Interfaces ===
// ==========================

pub trait RpcRequest: HasMethod + HasParams {}
pub trait RpcNotification: HasMethod + HasParams {}
/// Schema's `Result` interface: `{ _meta?: MetaObject, resultType: ResultType,
/// [key: string]: unknown }`. Bound to [`HasMeta`] + [`HasResultType`] —
/// arbitrary `[key: string]: unknown` extra keys are domain-specific and
/// expressed on the concrete struct itself.
pub trait RpcResult: HasMeta + HasResultType {}
pub trait JsonRpcRequestTrait: HasJsonRpcVersion + HasRequestId + RpcRequest {}
pub trait JsonRpcNotificationTrait: HasJsonRpcVersion + RpcNotification {}

/// `JSONRPCResultResponse` per schema — successful response carrying `result`.
pub trait JsonRpcResultResponseTrait:
    HasJsonRpcVersion + HasRequestId + HasResult + Serialize
{
    fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }
}

/// `JSONRPCErrorResponse` per schema — error response with optional `id`.
pub trait JsonRpcErrorResponseTrait:
    HasJsonRpcVersion + HasOptionalRequestId + HasErrorObject + Serialize
{
    fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }
}

// ==========================
// === Param Specialisations ===
// ==========================

pub trait HasRequestIdParam: Params {
    fn request_id(&self) -> &Value;
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

// ==========================
// === Typed Traits from MCP Spec ===
// ==========================

pub trait HasCancelledParams: HasRequestIdParam + HasReasonParam {}
pub trait CancelledNotification: RpcNotification + HasCancelledParams {}

// ---------------------- notifications/progress ------------------------

/// Trait for params of `notifications/progress`
pub trait HasProgressParams: Params {
    fn progress_token(&self) -> &ProgressToken;
    // `progress` is `f64` per spec — fractional progress in `[0.0, 1.0]`.
    fn progress(&self) -> f64;
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
        "notifications/resources/list_changed"
    }
}

pub trait HasReadResourceRequestParams: Params {
    fn uri(&self) -> &String;
}

pub trait ReadResourceRequest: JsonRpcRequestTrait + HasReadResourceRequestParams {
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

pub trait HasGetPromptRequestParams: Params {
    fn name(&self) -> &String;
    fn arguments(&self) -> Option<&HashMap<String, String>>; // MCP spec: { [key: string]: string }
}

pub trait GetPromptRequest: JsonRpcRequestTrait + HasGetPromptRequestParams {
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
        "notifications/prompts/list_changed"
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

pub trait HasCallToolRequestParams: Params {
    fn name(&self) -> &String;
    /// Tool arguments map. Schema's `arguments?: { [key: string]: unknown }`.
    fn arguments(&self) -> Option<&HashMap<String, Value>>;
    fn meta(&self) -> Option<&HashMap<String, Value>>;
}

pub trait CallToolRequest: JsonRpcRequestTrait + HasCallToolRequestParams {
    fn method(&self) -> &str {
        "tools/call"
    }
}

pub trait CallToolResult: RpcResult {
    fn content(&self) -> &Vec<ToolResult>;
    fn is_error(&self) -> Option<bool>;
    /// Structured content that matches the tool's `outputSchema`.
    fn structured_content(&self) -> Option<&Value>;
}

pub trait ToolListChangedNotification: JsonRpcNotificationTrait {
    fn method(&self) -> &str {
        "notifications/tools/list_changed"
    }
}

// ---------------------- sampling/createMessage ------------------------

pub trait HasCreateMessageRequestParams: Params {
    fn messages(&self) -> &Vec<SamplingMessage>;
    fn model_preferences(&self) -> Option<&ModelPreferences>;
    fn system_prompt(&self) -> Option<&String>;
    fn include_context(&self) -> Option<&String>;
    fn temperature(&self) -> Option<&f64>;
    fn max_tokens(&self) -> u32;
    fn stop_sequences(&self) -> Option<&Vec<String>>;
    fn metadata(&self) -> Option<&Value>;
}

pub trait CreateMessageRequest: JsonRpcRequestTrait + HasCreateMessageRequestParams {
    fn method(&self) -> &str {
        "sampling/createMessage"
    }
}

/// `CreateMessageResult` — per schema `extends SamplingMessage`, NOT `Result`.
/// Bound to [`HasMeta`] only (not [`RpcResult`]) because it has no
/// `resultType` discriminator.
pub trait CreateMessageResult: HasMeta {
    fn role(&self) -> &Role;
    fn content(&self) -> &SamplingMessageContent;
    fn model(&self) -> &String;
    fn stop_reason(&self) -> Option<&String>;
}

// ---------------------- completion/complete ------------------------

/// The `params` object for `completion/complete`
pub trait HasCompleteRequestParams: Params {
    /// The prompt or resource reference to complete against.
    fn reference(&self) -> &Value;
    /// The name/value pair to complete.
    fn argument(&self) -> &CompleteArgument;
    /// Optional additional context.
    fn context(&self) -> Option<&Value>;
}

/// The JSON-RPC request for `completion/complete`
pub trait CompleteRequestTrait: JsonRpcRequestTrait + HasCompleteRequestParams {
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
    fn resource_templates(&self) -> &Vec<crate::resources::ResourceTemplate>;
    fn next_cursor(&self) -> Option<&Cursor>;
}

// ---------------------- roots/list ------------------------

pub trait HasListRootsParams: Params {}

pub trait ListRootsRequest: JsonRpcRequestTrait + HasListRootsParams {
    fn method(&self) -> &str {
        "roots/list"
    }
}

// `ListRootsResult` is `{roots: Root[]}` per the DRAFT-2026-v1 schema — bare,
// no `_meta`, no `resultType`. The `RpcResult: HasMeta + HasResultType` bound doesn't
// fit, so the trait is plain.
pub trait ListRootsResult {
    fn roots(&self) -> &Vec<Root>;
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

// `ElicitResult` is `{action, content?}` per the schema — bare, no `_meta`,
// no `resultType`. The `RpcResult: HasMeta + HasResultType` bound doesn't fit.
pub trait ElicitResult {
    fn action(&self) -> &Value;
    fn content(&self) -> Option<&HashMap<String, Value>>;
}

// ---------------------- trait-based parameter extraction ------------------------

/// Trait for extracting parameters from RequestParams using trait constraints
pub trait ParamExtractor<T: Params> {
    type Error;

    /// Extract parameters from RequestParams using trait-based conversion
    fn extract(params: turul_rpc::RequestParams) -> Result<T, Self::Error>;
}

/// Trait for serde-based parameter extraction (simpler cases)
pub trait SerdeParamExtractor<T: Params> {
    type Error;

    /// Extract parameters using serde deserialization
    fn extract_serde(params: turul_rpc::RequestParams) -> Result<T, Self::Error>;
}

/// Trait for field-by-field parameter extraction (complex cases)
pub trait FieldParamExtractor<T: Params> {
    type Error;

    /// Extract parameters field by field with validation
    fn extract_fields(params: turul_rpc::RequestParams) -> Result<T, Self::Error>;
}
