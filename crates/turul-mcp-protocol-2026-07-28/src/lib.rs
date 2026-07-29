//! # Model Context Protocol (MCP) — 2026-07-28
//!
//! Faithful 1:1 Rust implementation of the upstream MCP schema
//! ([`schema.ts` vendored at `schema/schema.ts`](../../../schema/schema.ts)).
//! Wire-version string is [`MCP_VERSION`] = `"2026-07-28"`. It is the only
//! version literal this crate emits or accepts; the pre-finalization draft
//! literal is rejected (see [`version::McpVersion`]).
//!
//! Every `export interface`/`export type`/`export const` in the vendored
//! `schema/schema.ts` has a corresponding Rust binding here. Every
//! binding has a compliance test in `tests/compliance.rs`. The full
//! authoritative coverage source is `docs/plans/2026-07-28-spec-compliance.md`.
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
//!   tool/resource/prompt → `-32602`; MCP-specific `-32020`/`-32021`/`-32022`
//!   (header mismatch / capability / version negotiation failures) allocated
//!   sequentially from the schema's spec-reserved `-32020..-32099` range.
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
//!   [`HTTP_HEADER_METHOD`], [`HTTP_HEADER_NAME`], [`HTTP_HEADER_PARAM_PREFIX`].
//!
//! The upstream `schema.ts` has finalized its wire-version literal, and the
//! vendored `schema/schema.ts` is taken from the released
//! `schema/2026-07-28/` upstream path. That directory receives only errata
//! against the released spec; `schema/README.md` records the pin provenance
//! and the re-pin procedure.
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
pub mod sampling;
pub mod schema;
pub mod subscriptions;
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
/// at `schema/2026-07-28/examples`). Gated behind the `compliance` Cargo feature
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
    ElicitAction, ElicitRequest, ElicitRequestFormParams, ElicitResult, ElicitationBuilder,
    ElicitationSchema, PrimitiveSchemaDefinition, StringFormat,
};
// MCP-specific params/result shapes (what goes inside the wire envelope's
// `params`/`result` fields). Envelopes themselves come from `turul-rpc` below.
pub use caching::{CacheScope, CacheableResult};
pub use discover::{
    DiscoverRequest, DiscoverResult, DiscoverResultResponse, SERVER_DISCOVER_METHOD,
};
pub use headers::{
    ERROR_CODE_HEADER_MISMATCH, HTTP_HEADER_METHOD, HTTP_HEADER_NAME, HTTP_HEADER_PARAM_PREFIX,
    HTTP_HEADER_PROTOCOL_VERSION, MCP_PARAM_BASE64_PREFIX, MCP_PARAM_BASE64_SUFFIX,
    X_MCP_HEADER_SCHEMA_KEY,
};
pub use input_required::{
    InputRequest, InputRequests, InputRequiredResult, InputResponse, InputResponseRequestParams,
    InputResponses,
};
pub use json_rpc::{JSONRPC_VERSION, PaginatedRequestParams, RequestParams};
pub use meta::{Cursor as MetaCursor, ProgressToken};
#[allow(deprecated)] // META_KEY_LOG_LEVEL re-exported through the SEP-2577 migration window
pub use meta::{
    META_KEY_BAGGAGE, META_KEY_CLIENT_CAPABILITIES, META_KEY_CLIENT_INFO, META_KEY_LOG_LEVEL,
    META_KEY_PROTOCOL_VERSION, META_KEY_SUBSCRIPTION_ID, META_KEY_TRACEPARENT, META_KEY_TRACESTATE,
};
pub use notifications::{
    CancelledNotification, Notification, NotificationParams, ProgressNotification,
    ProgressNotificationParams, ProgressTokenValue, PromptListChangedNotification,
    ResourceListChangedNotification, ResourceUpdatedNotification,
    ResourceUpdatedNotificationParams, ToolListChangedNotification,
};
pub use result_type::ResultType;
pub use subscriptions::{
    SUBSCRIPTIONS_ACKNOWLEDGED_METHOD, SUBSCRIPTIONS_LISTEN_METHOD, SubscriptionFilter,
    SubscriptionsAcknowledgedNotification, SubscriptionsAcknowledgedNotificationParams,
    SubscriptionsListenRequest, SubscriptionsListenRequestParams, SubscriptionsListenResult,
    SubscriptionsListenResultMetaObject, SubscriptionsListenResultResponse,
};
// SEP-2577-deprecated re-exports kept available during the migration window.
#[allow(deprecated)]
pub use notifications::{LoggingMessageNotification, LoggingMessageNotificationParams};
pub use ping::{EmptyParams, EmptyResult};
pub use schema::JsonSchema;
pub use traits::{
    HasData, HasDataParam, HasErrorObject, HasMeta, HasMetaParam, HasNotificationMeta,
    HasOptionalRequestId,
    HasProgressTokenParam, HasRequestId, HasResultType, JsonRpcErrorResponseTrait,
    JsonRpcNotificationTrait, JsonRpcRequestTrait, JsonRpcResultResponseTrait, Params, RpcResult,
};

// JSON-RPC wire envelopes come from `turul-rpc` (0.2 schema-compliant types).
// We rename `turul_rpc::RequestParams` (the JSON-RPC envelope's `params` value:
// `Object | Array`) to `JsonRpcParams` so it doesn't collide with the MCP
// `RequestParams` interface (which is the named-properties shape that goes
// inside the `Object` variant).
pub use turul_rpc::RequestParams as JsonRpcParams;
pub use turul_rpc::{
    JsonRpcError, JsonRpcErrorCode, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest,
    JsonRpcResponse, JsonRpcSuccessResponse, JsonRpcVersion, JsonRpcWireMessage, RequestId,
    ResponseResult, error::JsonRpcErrorObject, parse_json_rpc_wire_message,
};

/// The MCP protocol version string this crate currently targets, exactly as it appears
/// on the wire in `LATEST_PROTOCOL_VERSION` of the upstream `schema.ts`.
///
/// The finalized schema emits the stable date literal `"2026-07-28"`. The
/// pre-finalization draft literal is rejected by [`McpVersion`], not accepted
/// — the transitional alias was retired.
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

    /// MCP `-32021` — server requires a client capability that was not declared
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

    /// MCP `-32022` — request's protocol version is not supported by the server.
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

    /// MRTR (SEP-2322): the handler needs client input before it can complete.
    ///
    /// NOT a wire error — this rides the error channel only because
    /// `McpResult` is the single return path available to tool/handler
    /// implementations. The `tools/call` handler converts it into a successful
    /// `InputRequiredResult` (`resultType: "input_required"`). Per schema, at
    /// least one of `input_requests` / `request_state` must be present.
    /// `request_state` is echoed verbatim by clients and MUST be treated as
    /// attacker-controlled on the retry — sign or encrypt it (e.g. HMAC) if it
    /// influences authorization.
    #[error("Input required (MRTR): handler needs client input before completing")]
    InputRequired {
        /// Requests the client must fulfill before retrying the original call.
        input_requests: Option<crate::input_required::InputRequests>,
        /// Opaque state blob the client echoes verbatim in the retry.
        request_state: Option<String>,
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

            // MCP-specific structured errors: MissingRequiredClientCapabilityError (-32021)
            // and UnsupportedProtocolVersionError (-32022) per the schema's allocated
            // -32020..-32099 spec-reserved range.
            McpError::MissingRequiredClientCapability { required } => {
                JsonRpcErrorObject::server_error(
                    -32021,
                    "Missing required client capability",
                    Some(serde_json::json!({ "requiredCapabilities": required })),
                )
            }
            McpError::UnsupportedProtocolVersion {
                supported,
                requested,
            } => JsonRpcErrorObject::server_error(
                -32022,
                "Unsupported protocol version",
                Some(serde_json::json!({
                    "supported": supported,
                    "requested": requested,
                })),
            ),

            // MRTR sentinel — the tools/call handler converts it to an
            // InputRequiredResult before dispatch ever serializes an error.
            // Reaching this arm means a handler emitted it on a method with
            // no MRTR conversion; surface as an internal error.
            McpError::InputRequired { .. } => JsonRpcErrorObject::internal_error(Some(
                "InputRequired escaped MRTR conversion (handler bug)".to_string(),
            )),

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

            // Validation errors. These codes are frozen legacy allocations in
            // the -32000..-32019 sub-range: 2026-07-28 forbids allocating new
            // codes there, so a new framework-internal code belongs outside the
            // JSON-RPC reserved range -32768..-32000 instead.
            McpError::ValidationError(msg) => JsonRpcErrorObject::server_error(
                -32014,
                &format!("Validation error: {}", msg),
                None,
            ),
            McpError::InvalidCapability(cap) => JsonRpcErrorObject::server_error(
                -32015,
                &format!("Invalid capability: {}", cap),
                None,
            ),
            McpError::VersionMismatch { expected, actual } => JsonRpcErrorObject::server_error(
                -32016,
                &format!(
                    "Protocol version mismatch: expected {}, got {}",
                    expected, actual
                ),
                None,
            ),

            // Configuration and session errors
            McpError::ConfigurationError(msg) => JsonRpcErrorObject::server_error(
                -32017,
                &format!("Configuration error: {}", msg),
                None,
            ),
            McpError::SessionError(msg) => {
                JsonRpcErrorObject::server_error(-32018, &format!("Session error: {}", msg), None)
            }

            // Transport and protocol layer errors
            McpError::TransportError(msg) => {
                JsonRpcErrorObject::server_error(-32019, &format!("Transport error: {}", msg), None)
            }
            McpError::JsonRpcProtocolError(msg) => JsonRpcErrorObject::server_error(
                -32000,
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

#[cfg(test)]
mod mcp_error_code_partition {
    //! 2026-07-28 partitions JSON-RPC's server-error range: `-32000..-32019`
    //! is the legacy sub-range — new codes MUST NOT be allocated in it and new
    //! implementations SHOULD NOT use it at all — and `-32020..-32099` is
    //! reserved for the specification, allocated sequentially. New codes for
    //! purposes the specification does not define SHOULD be allocated outside
    //! the JSON-RPC reserved range `-32768..-32000`. Every spec-registered
    //! structured error MUST use its assigned number. See
    //! [Error Codes](https://modelcontextprotocol.io/specification/2026-07-28/basic/index#error-codes).
    use super::McpError;

    /// Framework-internal codes already in use when 2026-07-28 introduced the
    /// sub-range partition. Frozen: nothing new may join them, because the
    /// specification forbids allocating new codes in `-32000..-32019`.
    const LEGACY_ALLOCATIONS: [i64; 11] = [
        -32000, -32010, -32011, -32012, -32013, -32014, -32015, -32016, -32017, -32018, -32019,
    ];

    #[test]
    fn missing_required_client_capability_uses_spec_code() {
        let err = McpError::MissingRequiredClientCapability {
            required: serde_json::json!({}),
        };
        assert_eq!(err.to_error_object().code, -32021);
    }

    #[test]
    fn unsupported_protocol_version_uses_spec_code() {
        let err = McpError::UnsupportedProtocolVersion {
            supported: vec!["2026-07-28".to_string()],
            requested: "2099-01-01".to_string(),
        };
        assert_eq!(err.to_error_object().code, -32022);
    }

    #[test]
    fn header_mismatch_constant_uses_spec_code() {
        assert_eq!(crate::headers::ERROR_CODE_HEADER_MISMATCH, -32020);
    }

    /// A framework-internal code is acceptable only if it is one of the frozen
    /// legacy allocations or sits outside the JSON-RPC reserved range
    /// entirely. The `-32000..-32019` sub-range is closed to new allocations,
    /// not the recommended home for them.
    #[test]
    fn framework_internal_errors_are_legacy_allocations_or_outside_the_reserved_range() {
        let cases: Vec<(McpError, &str)> = vec![
            (
                McpError::ToolExecutionError("x".into()),
                "ToolExecutionError",
            ),
            (
                McpError::ResourceAccessDenied("x".into()),
                "ResourceAccessDenied",
            ),
            (
                McpError::ResourceExecutionError("x".into()),
                "ResourceExecutionError",
            ),
            (
                McpError::PromptExecutionError("x".into()),
                "PromptExecutionError",
            ),
            (McpError::ValidationError("x".into()), "ValidationError"),
            (McpError::InvalidCapability("x".into()), "InvalidCapability"),
            (
                McpError::VersionMismatch {
                    expected: "a".into(),
                    actual: "b".into(),
                },
                "VersionMismatch",
            ),
            (
                McpError::ConfigurationError("x".into()),
                "ConfigurationError",
            ),
            (McpError::SessionError("x".into()), "SessionError"),
            (McpError::TransportError("x".into()), "TransportError"),
            (
                McpError::JsonRpcProtocolError("x".into()),
                "JsonRpcProtocolError",
            ),
        ];
        for (err, name) in cases {
            let code = err.to_error_object().code;
            assert!(
                LEGACY_ALLOCATIONS.contains(&code) || !(-32768..=-32000).contains(&code),
                "{name} emits {code}, which is neither a frozen legacy \
                 allocation nor outside the JSON-RPC reserved range \
                 -32768..-32000. New codes MUST NOT be allocated in \
                 -32000..-32019 and MUST NOT be emitted from the spec-reserved \
                 -32020..-32099; allocate outside the reserved range instead"
            );
        }
    }

    #[test]
    fn no_two_framework_internal_errors_share_a_code() {
        let codes = [
            McpError::ToolExecutionError("x".into()).to_error_object().code,
            McpError::ResourceAccessDenied("x".into())
                .to_error_object()
                .code,
            McpError::ResourceExecutionError("x".into())
                .to_error_object()
                .code,
            McpError::PromptExecutionError("x".into())
                .to_error_object()
                .code,
            McpError::ValidationError("x".into()).to_error_object().code,
            McpError::InvalidCapability("x".into())
                .to_error_object()
                .code,
            McpError::VersionMismatch {
                expected: "a".into(),
                actual: "b".into(),
            }
            .to_error_object()
            .code,
            McpError::ConfigurationError("x".into())
                .to_error_object()
                .code,
            McpError::SessionError("x".into()).to_error_object().code,
            McpError::TransportError("x".into()).to_error_object().code,
            McpError::JsonRpcProtocolError("x".into())
                .to_error_object()
                .code,
        ];
        let mut sorted = codes.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), codes.len(), "duplicate internal error code in {codes:?}");
    }
}

#[cfg(test)]
mod sep_2577_marker_tripwire {
    /// SEP-2577 absorbed `#[deprecated]` markers onto the Roots, Sampling,
    /// and Logging surfaces. This tripwire fails if a refactor drops them:
    /// each named item's declaration must be preceded by a deprecation
    /// attribute within the few lines above it.
    #[test]
    fn deprecation_markers_are_present() {
        for (source, items) in [
            (
                include_str!("roots.rs"),
                &["pub struct Root ", "pub struct ListRootsRequest"][..],
            ),
            (
                include_str!("sampling.rs"),
                &["pub struct CreateMessageRequest ", "pub struct ModelHint"][..],
            ),
            (include_str!("logging.rs"), &["pub enum LoggingLevel"][..]),
        ] {
            for item in items {
                let pos = source
                    .find(item)
                    .unwrap_or_else(|| panic!("{item} not found"));
                let preceding = &source[pos.saturating_sub(2000)..pos];
                assert!(
                    preceding.contains("#[deprecated"),
                    "{item} must carry a #[deprecated] marker (SEP-2577)"
                );
            }
        }
    }
}
