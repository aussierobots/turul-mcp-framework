//! Traits for JSON-RPC types as per MCP specification (2026-07-28).
//!
//! These model the schema's own interface inheritance, which Rust has no
//! equivalent for: `ProgressNotificationParams extends NotificationParams` in
//! TypeScript becomes a `Has*` trait here. Names follow the schema's interface
//! names (`CallToolRequest`, `CallToolResult`, etc.), so the mapping stays 1:1.
//!
//! They are protocol traits, not authoring traits. The traits a tool or resource
//! implements — `HasInputSchema`, `HasExecution`, `HasIcons` and the rest — live
//! in `turul-mcp-builders`, and no name is defined in both places.

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

// ModelPreferences (and LogLevel below) are SEP-2577-deprecated; the trait
// surface retains them through the migration window.
#[allow(deprecated)]
use crate::{
    completion::CompleteArgument,
    logging::LogLevel,
    meta::{Cursor, ProgressToken},
    prompts::{Prompt, PromptMessage},
    resources::Resource,
    sampling::{ModelPreferences, Role},
    tools::{Tool, ToolResult},
};
// Imports below reference SEP-2577-deprecated types used inside
// trait abstractions retained for the 12-month migration window.
#[allow(deprecated)]
use crate::{
    roots::Root,
    sampling::{SamplingMessage, SamplingMessageContent},
};

// JSON-RPC version constant
/// The JSON-RPC protocol version string, `"2.0"`, carried on every request and response envelope.
pub const JSONRPC_VERSION: &str = "2.0";

// ====================
// === Base Traits ====
// ====================

/// Marker for a typed `params` object of a request or notification.
pub trait Params {}

/// Required request id — present on requests and successful responses.
/// Schema: `RequestId = string | number`; we expose as borrowed `Value` for
/// permissive accept (the typed narrowing is a separate follow-up).
pub trait HasRequestId {
    /// Returns the request id.
    fn id(&self) -> &Value;
}

/// Optional request id — used by `JSONRPCErrorResponse` where the server may
/// omit `id` if it couldn't parse the original request's id.
pub trait HasOptionalRequestId {
    /// Returns the request id, or `None` when the sender could not determine it.
    fn id(&self) -> Option<&Value>;
}

/// Exposes the `result` payload of a `JSONRPCResultResponse`.
pub trait HasResult {
    /// Returns the result payload.
    fn result(&self) -> &dyn RpcResult;
}

/// Exposes the `jsonrpc` version member of a JSON-RPC envelope.
pub trait HasJsonRpcVersion {
    /// Returns the JSON-RPC version, `"2.0"` by default.
    fn version(&self) -> &str {
        JSONRPC_VERSION
    }
}

/// Exposes the `method` name of a request or notification.
pub trait HasMethod {
    /// Returns the method name.
    fn method(&self) -> &str;
}

/// Exposes the optional `params` of a request or notification.
pub trait HasParams {
    /// Returns the params, or `None` when the message carries none.
    fn params(&self) -> Option<&dyn Params>;
}

/// Exposes `Result._meta?: ResultMetaObject` — the result-side carrier, which
/// adds `io.modelcontextprotocol/serverInfo` on top of `MetaObject`. Borrowed,
/// typed — no JSON round-trip.
pub trait HasMeta {
    /// Returns the result `_meta`, or `None` when absent.
    fn meta(&self) -> Option<&crate::meta::ResultMetaObject>;
}

/// Exposes `Notification.params._meta?`. Separate from [`HasMeta`] because the
/// schema gives the two sides different carriers: results extend `MetaObject`
/// with `serverInfo`, notifications with `subscriptionId`. One trait returning
/// one type cannot model both.
pub trait HasNotificationMeta {
    /// Returns the notification params `_meta`, or `None` when absent.
    fn meta(&self) -> Option<&crate::meta::MetaObject>;
}

/// Exposes the `resultType` discriminator per the schema's
/// `Result.resultType: ResultType` field. Every spec-compliant result MUST
/// carry this; per the schema doc-comment, absent values default to `Complete`
/// for backward compatibility with pre-2026-07-28 servers.
pub trait HasResultType {
    /// Returns the result type discriminator.
    fn result_type(&self) -> crate::result_type::ResultType;
}

/// Escape hatch — flatten a typed result into a JSON-object map. Not part of
/// the [`RpcResult`] supertrait bound; consumers that need this should prefer
/// `serde_json::to_value`. Retained for the rare framework site that needs an
/// untyped view.
pub trait HasData {
    /// Returns the result flattened into a JSON-object map.
    fn data(&self) -> HashMap<String, Value>;
}

/// Exposes the `error` field on a `JSONRPCErrorResponse`. The schema's
/// `Error` interface maps to [`turul_rpc::error::JsonRpcErrorObject`]
/// (`{ code, message, data? }`).
pub trait HasErrorObject {
    /// Returns the JSON-RPC error object.
    fn error(&self) -> &turul_rpc::error::JsonRpcErrorObject;
}

// ==========================
// === Derived Interfaces ===
// ==========================

/// A JSON-RPC request body: a method name and optional params, without the `jsonrpc` and `id` envelope members.
pub trait RpcRequest: HasMethod + HasParams {}
/// A JSON-RPC notification body: a method name and optional params, without the `jsonrpc` envelope member.
pub trait RpcNotification: HasMethod + HasParams {}
/// Schema's `Result` interface: `{ _meta?: ResultMetaObject, resultType: ResultType,
/// [key: string]: unknown }`. Bound to [`HasMeta`] + [`HasResultType`] —
/// arbitrary `[key: string]: unknown` extra keys are domain-specific and
/// expressed on the concrete struct itself.
pub trait RpcResult: HasMeta + HasResultType {}
/// A complete JSON-RPC request: version, request id, method and params.
pub trait JsonRpcRequestTrait: HasJsonRpcVersion + HasRequestId + RpcRequest {}
/// A complete JSON-RPC notification: version, method and params.
pub trait JsonRpcNotificationTrait: HasJsonRpcVersion + RpcNotification {}

/// `JSONRPCResultResponse` per schema — successful response carrying `result`.
pub trait JsonRpcResultResponseTrait:
    HasJsonRpcVersion + HasRequestId + HasResult + Serialize
{
    /// Serialises the response to JSON bytes.
    ///
    /// # Panics
    ///
    /// Panics if serialisation fails.
    fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }
}

/// `JSONRPCErrorResponse` per schema — error response with optional `id`.
pub trait JsonRpcErrorResponseTrait:
    HasJsonRpcVersion + HasOptionalRequestId + HasErrorObject + Serialize
{
    /// Serialises the response to JSON bytes.
    ///
    /// # Panics
    ///
    /// Panics if serialisation fails.
    fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }
}

// ==========================
// === Param Specialisations ===
// ==========================

/// Exposes a free-form `data` map carried in a params object.
pub trait HasDataParam: Params {
    /// Returns the data map.
    fn data(&self) -> &HashMap<String, Value>;
}

/// Exposes the optional `_meta` map carried in a params object.
pub trait HasMetaParam: Params {
    /// Returns the `_meta` map, or `None` when absent.
    fn meta(&self) -> Option<&HashMap<String, Value>>;
}

/// Exposes the optional `progressToken` a requester attached to ask for progress notifications.
pub trait HasProgressTokenParam: Params {
    /// Returns the progress token, or `None` when the requester did not ask for progress.
    fn progress_token(&self) -> Option<&ProgressToken>;
}

// ==========================
// === Typed Traits from MCP Spec ===
// ==========================

// `*Notification` traits below are bound on `RpcNotification` (which requires
// only `HasMethod + HasParams`), NOT on `JsonRpcNotificationTrait` (which would
// also require `HasJsonRpcVersion`). The concrete notification structs in
// `notifications.rs` intentionally carry only `method` + `params` — the
// `jsonrpc: "2.0"` envelope is added by wrapping in `JsonRpcNotification` at
// transport time. The `RpcNotification`-bound abstraction is satisfiable by
// the structs as-shipped.

// Notification traits below use a `*Trait` suffix to avoid name collision
// with the same-named structs in `notifications.rs` (e.g. trait
// `CancelledNotificationTrait` vs struct `CancelledNotification`). The
// schema-level interface names are reflected in the struct, not the trait.
//
// Has*Params traits are bound on `Params` and impl'd on the *Params struct
// (which carries the actual field bodies). The wire-level *Trait abstractions
// are bound on `RpcNotification` (HasMethod + HasParams) and impl'd on the
// outer notification struct.

// ---------------------- notifications/cancelled ------------------------

/// Field getters for the `params` of `notifications/cancelled`.
pub trait HasCancelledParams: Params {
    /// Schema: `requestId` — required. Must correspond to the ID of a
    /// request the client previously issued (the only server-sent form is
    /// closing a `subscriptions/listen` stream on stdio).
    fn request_id(&self) -> &turul_rpc::RequestId;
    /// Returns the optional human-readable reason for the cancellation.
    fn reason(&self) -> Option<&str>;
}

/// Identifies a `notifications/cancelled` notification by method.
pub trait CancelledNotificationTrait: RpcNotification {
    /// Returns the wire method name, `"notifications/cancelled"`.
    fn method_string(&self) -> &str {
        "notifications/cancelled"
    }
}

// ---------------------- notifications/progress ------------------------

/// Field getters for the `params` of `notifications/progress`.
pub trait HasProgressParams: Params {
    /// Returns the token identifying the operation this progress refers to.
    fn progress_token(&self) -> &ProgressToken;
    /// `progress` is `f64` per spec — fractional progress in `[0.0, 1.0]`.
    fn progress(&self) -> f64;
    /// Returns the total amount of work when known, in the same unit as `progress`, or `None`.
    fn total(&self) -> Option<f64>;
    /// Returns an optional human-readable description of the current progress.
    fn message(&self) -> Option<&str>;
}

/// Identifies a `notifications/progress` notification by method.
pub trait ProgressNotificationTrait: RpcNotification {
    /// Returns the wire method name, `"notifications/progress"`.
    fn method_string(&self) -> &str {
        "notifications/progress"
    }
}

// ---------------------- notifications/resources/list_changed ------------------------

/// Identifies a `notifications/resources/list_changed` notification by method.
pub trait ResourcesListChangedNotificationTrait: RpcNotification {
    /// Returns the wire method name, `"notifications/resources/list_changed"`.
    fn method_string(&self) -> &str {
        "notifications/resources/list_changed"
    }
}

// ---------------------- notifications/resources/updated ------------------------

/// Field getters for the `params` of `notifications/resources/updated`.
pub trait HasResourceUpdatedParams: Params {
    /// Returns the URI of the resource that changed.
    fn uri(&self) -> &str;
}

/// Identifies a `notifications/resources/updated` notification by method.
pub trait ResourceUpdatedNotificationTrait: RpcNotification {
    /// Returns the wire method name, `"notifications/resources/updated"`.
    fn method_string(&self) -> &str {
        "notifications/resources/updated"
    }
}

// ---------------------- notifications/prompts/list_changed ------------------------

/// Identifies a `notifications/prompts/list_changed` notification by method.
pub trait PromptListChangedNotificationTrait: RpcNotification {
    /// Returns the wire method name, `"notifications/prompts/list_changed"`.
    fn method_string(&self) -> &str {
        "notifications/prompts/list_changed"
    }
}

// ---------------------- notifications/tools/list_changed ------------------------

/// Identifies a `notifications/tools/list_changed` notification by method.
pub trait ToolListChangedNotificationTrait: RpcNotification {
    /// Returns the wire method name, `"notifications/tools/list_changed"`.
    fn method_string(&self) -> &str {
        "notifications/tools/list_changed"
    }
}

// ---------------------- notifications/message ------------------------

/// **Deprecated** per SEP-2577 — see [`crate::notifications::LoggingMessageNotification`].
#[deprecated(
    since = "0.4.0",
    note = "Deprecated per SEP-2577 (2026-07-28). \
            Replacement: stderr (stdio) or OpenTelemetry, plus per-request log-level opt-in. \
            Earliest removal: first release on/after 2027-07-28."
)]
/// Identifies a `notifications/message` notification by method.
pub trait LoggingMessageNotificationTrait: RpcNotification {
    /// Returns the wire method name, `"notifications/message"`.
    fn method_string(&self) -> &str {
        "notifications/message"
    }
}

// ---------------------- resources/list ------------------------

/// Field getters for the `params` of `resources/list`.
pub trait HasListResourcesParams: Params {
    /// Returns the pagination cursor, or `None` to request the first page.
    fn cursor(&self) -> Option<&Cursor>;
}

/// A `resources/list` request.
pub trait ListResourcesRequest: JsonRpcRequestTrait + HasListResourcesParams {
    /// Returns the wire method name, `"resources/list"`.
    fn method(&self) -> &str {
        "resources/list"
    }
}

/// The result of `resources/list`: one page of resources.
pub trait ListResourcesResult: RpcResult {
    /// Returns the resources on this page.
    fn resources(&self) -> &Vec<Resource>;
    /// Returns the cursor for the next page, or `None` when there are no more results.
    fn next_cursor(&self) -> Option<&Cursor>;
}

/// Field getters for the `params` of `resources/read`.
pub trait HasReadResourceRequestParams: Params {
    /// Returns the URI of the resource to read.
    fn uri(&self) -> &String;
}

/// A `resources/read` request.
pub trait ReadResourceRequest: JsonRpcRequestTrait + HasReadResourceRequestParams {
    /// Returns the wire method name, `"resources/read"`.
    fn method(&self) -> &str {
        "resources/read"
    }
}

/// The result of `resources/read`: the contents of the requested resource.
pub trait ReadResourceResult: RpcResult {
    /// Returns the contents of the resource.
    fn contents(&self) -> &Vec<crate::resources::ResourceContent>;
}

// ---------------------- prompts/list & get ------------------------

/// Field getters for the `params` of `prompts/list`.
pub trait HasListPromptsParams: Params {
    /// Returns the pagination cursor, or `None` to request the first page.
    fn cursor(&self) -> Option<&Cursor>;
}

/// A `prompts/list` request.
pub trait ListPromptsRequest: JsonRpcRequestTrait + HasListPromptsParams {
    /// Returns the wire method name, `"prompts/list"`.
    fn method(&self) -> &str {
        "prompts/list"
    }
}

/// The result of `prompts/list`: one page of prompts.
pub trait ListPromptsResult: RpcResult {
    /// Returns the prompts on this page.
    fn prompts(&self) -> &Vec<Prompt>;
    /// Returns the cursor for the next page, or `None` when there are no more results.
    fn next_cursor(&self) -> Option<&Cursor>;
}

/// Field getters for the `params` of `prompts/get`.
pub trait HasGetPromptRequestParams: Params {
    /// Returns the name of the prompt to render.
    fn name(&self) -> &String;
    /// Returns the string-valued arguments used to render the prompt, or `None`.
    fn arguments(&self) -> Option<&HashMap<String, String>>; // MCP spec: { [key: string]: string }
}

/// A `prompts/get` request.
pub trait GetPromptRequest: JsonRpcRequestTrait + HasGetPromptRequestParams {
    /// Returns the wire method name, `"prompts/get"`.
    fn method(&self) -> &str {
        "prompts/get"
    }
}

/// The result of `prompts/get`: the rendered prompt.
pub trait GetPromptResult: RpcResult {
    /// Returns an optional description of the prompt.
    fn description(&self) -> Option<&String>;
    /// Returns the messages that make up the rendered prompt.
    fn messages(&self) -> &Vec<PromptMessage>;
}

// ---------------------- tools/list & call ------------------------

/// Field getters for the `params` of `tools/list`.
pub trait HasListToolsParams: Params {
    /// Returns the pagination cursor, or `None` to request the first page.
    fn cursor(&self) -> Option<&Cursor>;
}

/// A `tools/list` request.
pub trait ListToolsRequest: JsonRpcRequestTrait + HasListToolsParams {
    /// Returns the wire method name, `"tools/list"`.
    fn method(&self) -> &str {
        "tools/list"
    }
}

/// The result of `tools/list`: one page of tools.
pub trait ListToolsResult: RpcResult {
    /// Returns the tools on this page.
    fn tools(&self) -> &Vec<Tool>;
    /// Returns the cursor for the next page, or `None` when there are no more results.
    fn next_cursor(&self) -> Option<&Cursor>;
}

/// Field getters for the `params` of `tools/call`.
pub trait HasCallToolRequestParams: Params {
    /// Returns the name of the tool to call.
    fn name(&self) -> &String;
    /// Tool arguments map. Schema's `arguments?: { [key: string]: unknown }`.
    fn arguments(&self) -> Option<&HashMap<String, Value>>;
    /// Returns the request `_meta` map, or `None` when absent.
    fn meta(&self) -> Option<&HashMap<String, Value>>;
}

/// A `tools/call` request.
pub trait CallToolRequest: JsonRpcRequestTrait + HasCallToolRequestParams {
    /// Returns the wire method name, `"tools/call"`.
    fn method(&self) -> &str {
        "tools/call"
    }
}

/// The result of `tools/call`: the tool output.
pub trait CallToolResult: RpcResult {
    /// Returns the unstructured content blocks produced by the tool.
    fn content(&self) -> &Vec<ToolResult>;
    /// Returns whether the tool call failed, or `None` when unset (treated as success).
    fn is_error(&self) -> Option<bool>;
    /// Structured content that matches the tool's `outputSchema`.
    fn structured_content(&self) -> Option<&Value>;
}

// ---------------------- sampling/createMessage ------------------------
//
// **Deprecated** per SEP-2577 — trait surface retained during the 12-month
// migration window. References deprecated types in its signature; suppression
// is scoped to this section.

#[allow(deprecated)]
#[deprecated(
    since = "0.4.0",
    note = "Deprecated per SEP-2577 (2026-07-28) with the Sampling surface. \
            Earliest removal: first release on/after 2027-07-28."
)]
/// Field getters for the `params` of `sampling/createMessage`.
pub trait HasCreateMessageRequestParams: Params {
    /// Returns the conversation messages to sample from.
    fn messages(&self) -> &Vec<SamplingMessage>;
    /// Returns the client model-selection preferences, or `None`.
    fn model_preferences(&self) -> Option<&ModelPreferences>;
    /// Returns the requested system prompt, or `None`.
    fn system_prompt(&self) -> Option<&String>;
    /// Returns which MCP server context to include with the request, or `None`.
    fn include_context(&self) -> Option<&String>;
    /// Returns the sampling temperature, or `None`.
    fn temperature(&self) -> Option<&f64>;
    /// Returns the maximum number of tokens to sample.
    fn max_tokens(&self) -> u32;
    /// Returns the sequences that stop generation, or `None`.
    fn stop_sequences(&self) -> Option<&Vec<String>>;
    /// Returns provider-specific metadata passed through to the model, or `None`.
    fn metadata(&self) -> Option<&HashMap<String, Value>>;
}

#[allow(deprecated)]
#[deprecated(
    since = "0.4.0",
    note = "Deprecated per SEP-2577 (2026-07-28) with the Sampling surface. \
            Earliest removal: first release on/after 2027-07-28."
)]
/// A `sampling/createMessage` request.
pub trait CreateMessageRequest: JsonRpcRequestTrait + HasCreateMessageRequestParams {
    /// Returns the wire method name, `"sampling/createMessage"`.
    fn method(&self) -> &str {
        "sampling/createMessage"
    }
}

/// `CreateMessageResult` — per schema `extends SamplingMessage`, NOT `Result`.
/// Bound to [`HasMeta`] only (not [`RpcResult`]) because it has no
/// `resultType` discriminator.
#[allow(deprecated)]
#[deprecated(
    since = "0.4.0",
    note = "Deprecated per SEP-2577 (2026-07-28) with the Sampling surface. \
            Earliest removal: first release on/after 2027-07-28."
)]
pub trait CreateMessageResult: HasMeta {
    /// Returns the role of the generated message.
    fn role(&self) -> &Role;
    /// Returns the content of the generated message.
    fn content(&self) -> &SamplingMessageContent;
    /// Returns the name of the model that generated the message.
    fn model(&self) -> &String;
    /// Returns why sampling stopped, or `None` when unreported.
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
    /// Returns the `completion` object holding the suggested values.
    fn completion(&self) -> &Value;
}

/// The JSON-RPC result for `completion/complete`
pub trait CompleteResult: RpcResult + HasCompletionResult {}

// ---------------------- templates/list ------------------------

/// Field getters for the `params` of `resources/templates/list`.
pub trait HasListResourceTemplatesParams: Params {
    /// Returns the pagination cursor, or `None` to request the first page.
    fn cursor(&self) -> Option<&Cursor>;
}

/// A `resources/templates/list` request.
pub trait ListResourceTemplatesRequest:
    JsonRpcRequestTrait + HasListResourceTemplatesParams
{
    /// Returns the wire method name, `"resources/templates/list"`.
    fn method(&self) -> &str {
        "resources/templates/list"
    }
}

/// The result of `resources/templates/list`: one page of resource templates.
pub trait ListResourceTemplatesResult: RpcResult {
    /// Returns the resource templates on this page.
    fn resource_templates(&self) -> &Vec<crate::resources::ResourceTemplate>;
    /// Returns the cursor for the next page, or `None` when there are no more results.
    fn next_cursor(&self) -> Option<&Cursor>;
}

// ---------------------- roots/list ------------------------

/// Marker for the `params` of `roots/list`, which carries no fields of its own.
pub trait HasListRootsParams: Params {}

/// A `roots/list` request.
pub trait ListRootsRequest: JsonRpcRequestTrait + HasListRootsParams {
    /// Returns the wire method name, `"roots/list"`.
    fn method(&self) -> &str {
        "roots/list"
    }
}

// `ListRootsResult` is `{roots: Root[]}` per the 2026-07-28 schema — bare,
// no `_meta`, no `resultType`. The `RpcResult: HasMeta + HasResultType` bound doesn't
// fit, so the trait is plain. **Deprecated** per SEP-2577 alongside the Roots
// surface; retained during the migration window.
/// The result of `roots/list`: the roots the client exposes.
#[allow(deprecated)]
pub trait ListRootsResult {
    /// Returns the roots the client exposes.
    fn roots(&self) -> &Vec<Root>;
}

// ---------------------- logging ------------------------

// The `logging/setLevel` RPC does not exist in 2026-07-28; per-request log
// level opt-in rides on `RequestMetaObject.log_level`.

// Field-getter traits implemented by `LoggingMessageParams` (the params for
// `notifications/message`).
#[allow(deprecated)]
#[deprecated(
    since = "0.4.0",
    note = "Deprecated per SEP-2577 (2026-07-28) along with the whole Logging surface."
)]
/// Exposes the severity `level` of a log message.
pub trait HasLevelParam: Params {
    /// Returns the severity level of the log message.
    #[allow(deprecated)]
    fn level(&self) -> &LogLevel;
}
/// Exposes the optional `logger` name that produced a log message.
pub trait HasLoggerParam: Params {
    /// Returns the logger name, or `None` when unnamed.
    fn logger(&self) -> Option<&String>;
}

// ---------------------- server/discover ------------------------

// `*RequestTrait` traits below are bound on `RpcRequest` (which requires only
// `HasMethod + HasParams`), NOT on `JsonRpcRequestTrait` (which would also
// require `HasJsonRpcVersion + HasRequestId`). The concrete request structs in
// each module intentionally carry only `method` + `params` — the
// `jsonrpc: "2.0"` envelope and `id` are added by wrapping in
// `turul_rpc::JsonRpcRequest` at transport time.

/// `server/discover` request — replaces the 2025-11-25 `initialize` handshake.
/// Params is the bare [`crate::json_rpc::RequestParams`]; capability negotiation
/// rides on `_meta: RequestMetaObject`. The trait gives consumers a uniform
/// way to detect a discover request by method.
pub trait DiscoverRequestTrait: RpcRequest {
    /// Returns the wire method name, `"server/discover"`.
    fn method_string(&self) -> &str {
        "server/discover"
    }
}

// ---------------------- subscriptions/listen ------------------------

/// Field-getter for `SubscriptionsListenRequestParams.notifications`
/// (the `SubscriptionFilter` declaring which notification types the client
/// opts in to on this stream).
pub trait HasSubscriptionsListenParams: Params {
    /// Returns the filter declaring which notification types the client opts in to on this stream.
    fn notifications(&self) -> &crate::subscriptions::SubscriptionFilter;
}

/// Identifies a `subscriptions/listen` request by method.
pub trait SubscriptionsListenRequestTrait: RpcRequest {
    /// Returns the wire method name, `"subscriptions/listen"`.
    fn method_string(&self) -> &str {
        "subscriptions/listen"
    }
}

// ---------------------- InputRequiredResult (SEP-2322) ------------------------

/// Field-getters for `InputRequiredResult` — the multi-round-trip
/// server-initiated request shape that replaces the 2025-11-25
/// server→client SSE stream.
///
/// Bound on [`HasResultType`] only (not on full [`RpcResult`]) because
/// `_meta` access is exposed through [`Self::meta`] below directly.
pub trait HasInputRequiredResult: HasResultType {
    /// Server-initiated requests the client must fulfill before retrying.
    fn input_requests(&self) -> Option<&crate::input_required::InputRequests>;
    /// Opaque state blob the client echoes back on the retry's
    /// `InputResponseRequestParams.request_state`.
    fn request_state(&self) -> Option<&str>;
    /// Optional `_meta` per the `Result` schema.
    fn meta(&self) -> Option<&crate::meta::ResultMetaObject>;
}

// ---------------------- elicitation ------------------------

/// Field getters for the `params` of `elicitation/create`.
pub trait HasElicitParams: Params {
    /// Returns the message shown to the user explaining what is being requested.
    fn message(&self) -> &String;
    /// Returns the JSON Schema describing the structure of the requested response.
    fn requested_schema(&self) -> &Value;
}

/// An `elicitation/create` request.
pub trait ElicitRequest: JsonRpcRequestTrait + HasElicitParams {
    /// Returns the wire method name, `"elicitation/create"`.
    fn method(&self) -> &str {
        "elicitation/create"
    }
}

// `ElicitResult` is `{action, content?}` per the schema — bare, no `_meta`,
// no `resultType`. The `RpcResult: HasMeta + HasResultType` bound doesn't fit.
/// The result of `elicitation/create`: the user action and any submitted content.
pub trait ElicitResult {
    /// Returns the action the user took: `accept`, `decline` or `cancel`.
    fn action(&self) -> &Value;
    /// Returns the submitted form data, or `None` unless the action is `accept`.
    fn content(&self) -> Option<&HashMap<String, Value>>;
}

// ---------------------- trait-based parameter extraction ------------------------

/// Trait for extracting parameters from RequestParams using trait constraints
pub trait ParamExtractor<T: Params> {
    /// The error returned when extraction fails.
    type Error;

    /// Extract parameters from RequestParams using trait-based conversion
    fn extract(params: turul_rpc::RequestParams) -> Result<T, Self::Error>;
}

/// Trait for serde-based parameter extraction (simpler cases)
pub trait SerdeParamExtractor<T: Params> {
    /// The error returned when extraction fails.
    type Error;

    /// Extract parameters using serde deserialization
    fn extract_serde(params: turul_rpc::RequestParams) -> Result<T, Self::Error>;
}

/// Trait for field-by-field parameter extraction (complex cases)
pub trait FieldParamExtractor<T: Params> {
    /// The error returned when extraction fails.
    type Error;

    /// Extract parameters field by field with validation
    fn extract_fields(params: turul_rpc::RequestParams) -> Result<T, Self::Error>;
}
