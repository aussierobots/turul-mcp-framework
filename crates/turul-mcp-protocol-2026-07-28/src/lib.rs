//! # Model Context Protocol (MCP) — 2026-07-28
//!
//! Faithful 1:1 Rust implementation of the upstream MCP schema
//! ([`schema.ts` vendored at `schema/draft-schema.ts`](../../../schema/draft-schema.ts)).
//! Wire-version string is [`MCP_VERSION`] = `"2026-07-28"` (the pre-finalization
//! draft literal `"DRAFT-2026-v1"` is still accepted on deserialize for back-compat).
//!
//! Every `export interface`/`export type`/`export const` in the vendored
//! `schema/draft-schema.ts` has a corresponding Rust binding here. Every
//! binding has a compliance test in `tests/compliance.rs`. The full
//! per-symbol coverage map is `docs/plans/2026-07-28-schema-coverage-matrix.md`.
//!
//! ## Surface area
//!
//! - **Stateless RPC** — per-request capability negotiation via
//!   [`meta::RequestMetaObject`] (`io.modelcontextprotocol/protocolVersion`,
//!   `clientInfo`, `clientCapabilities`). [SEP-2567], [SEP-2575].
//! - **`server/discover`** — [`discover::DiscoverRequest`] /
//!   [`discover::DiscoverResult`] for server capability advertisement.
//! - **Multi round-trip requests** ([SEP-2322]) — [`input_required`] module:
//!   `InputRequest`/`InputResponse` pairs, `InputRequiredResult` with
//!   `requestState` opaque echo, `InputResponseRequestParams` mixin embedded
//!   in `tools/call`, `resources/read`, `prompts/get` params.
//! - **Unified subscription stream** —
//!   [`subscriptions::SubscriptionsListenRequest`] with opt-in
//!   `SubscriptionFilter` and `SubscriptionsAcknowledgedNotification`.
//! - **Caching mixin** ([SEP-2549]) — [`caching::CacheableResult`] (`ttlMs` +
//!   `cacheScope`) required on every list/read result.
//! - **JSON Schema 2020-12** ([SEP-2106]) — [`tools::ToolSchema`] for tool
//!   `inputSchema` (root `type: "object"`); [`tools::ToolOutputSchema`] for
//!   unrestricted `outputSchema`.
//! - **Result discrimination** — [`result_type::ResultType`] required on every
//!   `Result`.
//! - **Extensions** ([SEP-2133]) — `extensions` map on capabilities; extension
//!   *types* live in separate `turul-mcp-ext-*` crates, not in this crate.
//! - **Error codes** ([SEP-2164]) — JSON-RPC standard codes; missing
//!   tool/resource/prompt → `-32602`; MCP-specific `-32003`/`-32004` for
//!   capability/version negotiation failures.
//!
//! ## `_meta` key + HTTP header constants
//!
//! Typed source-of-truth constants for spelling:
//!
//! - Schema-declared `_meta` keys: [`META_KEY_PROTOCOL_VERSION`],
//!   [`META_KEY_CLIENT_INFO`], [`META_KEY_CLIENT_CAPABILITIES`],
//!   [`META_KEY_LOG_LEVEL`].
//! - Convention `_meta` keys: [`META_KEY_TRACEPARENT`], [`META_KEY_TRACESTATE`],
//!   [`META_KEY_BAGGAGE`] (W3C Trace Context, [SEP-414]),
//!   [`META_KEY_SUBSCRIPTION_ID`] (subscription tagging).
//! - Streamable HTTP headers ([SEP-2243]): [`HTTP_HEADER_PROTOCOL_VERSION`],
//!   [`HTTP_HEADER_METHOD`], [`HTTP_HEADER_NAME`], [`HTTP_HEADER_CUSTOM_PREFIX`].
//!
//! The upstream `schema.ts` has finalized the wire-version literal to
//! `"2026-07-28"` (was `"DRAFT-2026-v1"` pre-finalization). The vendored
//! `schema/draft-schema.ts` still lives under the `schema/draft/` upstream path
//! and may continue to receive field-level revisions; `schema/README.md`
//! records the pin provenance and re-pin procedure.
//!
//! [SEP-2106]: https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/seps/2106-tool-output-schema.md
//! [SEP-2133]: https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/seps/2133-extensions.md
//! [SEP-2164]: https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/seps/2164-standard-error-codes.md
//! [SEP-2322]: https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/seps/2322-MRTR.md
//! [SEP-2549]: https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/seps/2549-TTL-for-list-results.md
//! [SEP-2567]: https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/seps/2567-sessionless-mcp.md
//! [SEP-2575]: https://github.com/modelcontextprotocol/modelcontextprotocol/pull/2575
//! [SEP-2663]: https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/seps/2663-tasks-extension.md
//!
//! ## `_meta` field usage
//!
//! Two distinct `_meta` carriers:
//!
//! - [`meta::RequestMetaObject`] — typed shape on every `Request.params._meta`.
//!   Carries the required `io.modelcontextprotocol/protocolVersion`,
//!   `clientInfo`, `clientCapabilities` plus optional `progressToken` and
//!   `logLevel`. Arbitrary namespaced keys ride along on `extra`.
//! - [`meta::MetaObject`] — loose `HashMap<String, Value>` for
//!   `Notification.params._meta` and `Result._meta`.

pub mod caching;
pub mod completion;
pub mod content;
pub mod discover;
pub mod elicitation;
pub mod headers;
pub mod icons;
pub mod initialize;
pub mod input_required;
pub mod json_rpc;
pub mod logging;
pub mod meta;
pub mod notifications;
pub mod param_extraction;
pub mod ping;
pub mod prelude;
pub mod prompts;
pub mod resources;
pub mod result_type;
pub mod roots;
pub mod subscriptions;
pub mod sampling;
pub mod schema;
pub mod tools;
pub mod traits;
pub mod version;

// Re-export key content types for convenience
pub use content::{
    BlobResourceContents, ContentBlock, ResourceContents, ResourceReference, TextResourceContents,
};
// Re-export key meta types for convenience
pub use meta::{Annotations, MetaObject, RequestMetaObject};

/// Bidirectional wire-format compliance harness against the upstream MCP spec's
/// canonical example JSON fixtures (`modelcontextprotocol/modelcontextprotocol`
/// at `schema/draft/examples`). Gated behind the `compliance` Cargo feature
/// (default-off) — adds no code to the published library surface.
///
/// Entry points: `tests/upstream_fixtures.rs` (build-time gate) and
/// `src/bin/compliance.rs` (runtime CLI). Both call the same
/// [`compliance::roundtrip::run_all`] so a green test guarantees a green binary.
#[cfg(feature = "compliance")]
pub mod compliance;

// Re-export main types
pub use icons::{Icon, IconTheme};
pub use initialize::{ClientCapabilities, Implementation, ServerCapabilities};
pub use prompts::{
    GetPromptRequest, GetPromptResult, ListPromptsRequest, ListPromptsResult, Prompt,
    PromptArgument, PromptMessage,
};
pub use resources::{
    ListResourcesRequest, ListResourcesResult, ReadResourceRequest, ReadResourceResult, Resource,
    ResourceContent,
};
pub use tools::{
    CallToolRequest, CallToolResult, ListToolsRequest, ListToolsResult, Tool, ToolOutputSchema,
    ToolResult, ToolSchema,
};
pub use version::McpVersion;
// ResourceTemplate functionality is now part of resources module
// pub use resources::{ResourceTemplate, ListResourceTemplatesRequest, ListResourceTemplatesResult};
pub use elicitation::{
    ElicitAction, ElicitRequestFormParams, ElicitRequest, ElicitResult, ElicitationBuilder,
    ElicitationSchema, PrimitiveSchemaDefinition, StringFormat,
};
// MCP-specific params/result shapes (what goes inside the wire envelope's
// `params`/`result` fields). Envelopes themselves come from `turul-rpc` below.
pub use json_rpc::{
    PaginatedRequestParams, RequestParams, JSONRPC_VERSION,
};
pub use caching::{CacheScope, CacheableResult};
pub use headers::{
    HTTP_HEADER_CUSTOM_PREFIX, HTTP_HEADER_METHOD, HTTP_HEADER_NAME, HTTP_HEADER_PROTOCOL_VERSION,
};
pub use meta::{
    META_KEY_BAGGAGE, META_KEY_CLIENT_CAPABILITIES, META_KEY_CLIENT_INFO, META_KEY_LOG_LEVEL,
    META_KEY_PROTOCOL_VERSION, META_KEY_SUBSCRIPTION_ID, META_KEY_TRACEPARENT, META_KEY_TRACESTATE,
};
pub use discover::{
    DiscoverRequest, DiscoverResult, DiscoverResultResponse, SERVER_DISCOVER_METHOD,
};
pub use input_required::{
    InputRequest, InputRequests, InputRequiredResult, InputResponse, InputResponseRequestParams,
    InputResponses,
};
pub use meta::{Cursor as MetaCursor, ProgressToken};
pub use result_type::ResultType;
pub use subscriptions::{
    SubscriptionFilter, SubscriptionsAcknowledgedNotification, SubscriptionsAcknowledgedNotificationParams,
    SubscriptionsListenRequestParams, SubscriptionsListenRequest, SUBSCRIPTIONS_ACKNOWLEDGED_METHOD,
    SUBSCRIPTIONS_LISTEN_METHOD,
};
pub use notifications::{
    CancelledNotification, ElicitationCompleteNotification, Notification, NotificationParams,
    ProgressNotification, ProgressNotificationParams, ProgressTokenValue,
    PromptListChangedNotification, ResourceListChangedNotification, ResourceUpdatedNotification,
    ResourceUpdatedNotificationParams, ToolListChangedNotification,
};
// SEP-2577-deprecated re-exports kept available during the migration window.
#[allow(deprecated)]
pub use notifications::{LoggingMessageNotification, LoggingMessageNotificationParams};
pub use ping::{EmptyParams, EmptyResult};
pub use schema::JsonSchema;
pub use traits::{
    HasData, HasDataParam, HasErrorObject, HasMeta, HasMetaParam, HasOptionalRequestId,
    HasProgressTokenParam, HasRequestId, HasResultType, JsonRpcErrorResponseTrait,
    JsonRpcNotificationTrait, JsonRpcRequestTrait, JsonRpcResultResponseTrait, Params, RpcResult,
};

// JSON-RPC wire envelopes come from `turul-rpc` (0.2 schema-compliant types).
// We rename `turul_rpc::RequestParams` (the JSON-RPC envelope's `params` value:
// `Object | Array`) to `JsonRpcParams` so it doesn't collide with the MCP
// `RequestParams` interface (which is the named-properties shape that goes
// inside the `Object` variant).
pub use turul_rpc::{
    JsonRpcError, JsonRpcErrorCode, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest,
    JsonRpcResponse, JsonRpcSuccessResponse, JsonRpcVersion, JsonRpcWireMessage, RequestId,
    ResponseResult, error::JsonRpcErrorObject, parse_json_rpc_wire_message,
};
pub use turul_rpc::RequestParams as JsonRpcParams;

/// The MCP protocol version string this crate currently targets, exactly as it appears
/// on the wire in `LATEST_PROTOCOL_VERSION` of the upstream `schema.ts`.
///
/// The finalized schema emits the stable date literal `"2026-07-28"` (the
/// pre-finalization draft emitted `"DRAFT-2026-v1"`, still accepted on
/// deserialize by [`McpVersion`] for back-compat).
pub const MCP_VERSION: &str = "2026-07-28";

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

    /// MCP `-32003` — server requires a client capability that was not declared
    /// in the request's `clientCapabilities`. See draft schema `MissingRequiredClientCapabilityError`.
    ///
    /// Wire `data` shape: `{"requiredCapabilities": <ClientCapabilities>}`. The
    /// `required` field is `serde_json::Value` transitionally — to be retyped
    /// to `ClientCapabilities` from `discover` when that integration lands.
    #[error("Missing required client capability")]
    MissingRequiredClientCapability {
        /// Capabilities the server requires from the client to process this request.
        required: serde_json::Value,
    },

    /// MCP `-32004` — request's protocol version is not supported by the server.
    /// See draft schema `UnsupportedProtocolVersionError`.
    ///
    /// Wire `data` shape: `{"supported": [..], "requested": ".."}`.
    #[error("Unsupported protocol version: requested {requested}, supported {supported:?}")]
    UnsupportedProtocolVersion {
        /// Protocol versions the server supports. The client should choose a
        /// mutually supported version from this list and retry.
        supported: Vec<String>,
        /// The protocol version that was requested by the client.
        requested: String,
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
    pub fn to_error_object(&self) -> turul_rpc::error::JsonRpcErrorObject {
        use turul_rpc::error::JsonRpcErrorObject;

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

            // Not-found errors map to JSON-RPC standard -32602 (Invalid Params) per the
            // draft schema's `InvalidParamsError` doc and SEP-2164: unknown tool/prompt/resource
            // names are treated as invalid parameters, not custom MCP server errors.
            McpError::ToolNotFound(name) => {
                JsonRpcErrorObject::invalid_params(&format!("Unknown tool: {}", name))
            }
            McpError::ResourceNotFound(uri) => {
                JsonRpcErrorObject::invalid_params(&format!("Unknown resource: {}", uri))
            }
            McpError::PromptNotFound(name) => {
                JsonRpcErrorObject::invalid_params(&format!("Unknown prompt: {}", name))
            }

            // MCP-specific structured errors: MissingRequiredClientCapabilityError (-32003)
            // and UnsupportedProtocolVersionError (-32004) per DRAFT-2026-v1 schema.
            McpError::MissingRequiredClientCapability { required } => {
                JsonRpcErrorObject::server_error(
                    -32003,
                    "Missing required client capability",
                    Some(serde_json::json!({ "requiredCapabilities": required })),
                )
            }
            McpError::UnsupportedProtocolVersion {
                supported,
                requested,
            } => JsonRpcErrorObject::server_error(
                -32004,
                "Unsupported protocol version",
                Some(serde_json::json!({
                    "supported": supported,
                    "requested": requested,
                })),
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
        id: Option<turul_rpc::RequestId>,
    ) -> turul_rpc::JsonRpcError {
        turul_rpc::JsonRpcError::new(id, self.to_error_object())
    }

    /// Legacy method for backward compatibility - use to_error_object() instead
    #[deprecated(note = "Use to_error_object() instead for cleaner architecture")]
    pub fn to_json_rpc_error(&self) -> turul_rpc::error::JsonRpcErrorObject {
        self.to_error_object()
    }
}

// Implement the ToJsonRpcError trait for MCP errors
impl turul_rpc::r#async::ToJsonRpcError for McpError {
    fn to_error_object(&self) -> turul_rpc::error::JsonRpcErrorObject {
        // Delegate to our existing type-safe implementation
        McpError::to_error_object(self)
    }
}
