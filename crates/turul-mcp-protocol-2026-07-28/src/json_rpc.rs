//! JSON-RPC 2.0 envelopes for MCP DRAFT-2026-v1
//!
//! Provides the wire-level envelope types: `JsonRpcRequest`, `JsonRpcNotification`,
//! `JsonRpcResponse`, `JsonRpcError`, `JsonRpcMessage` (untagged union).
//!
//! Maps to the draft schema's JSON-RPC envelope section (`schema/draft-schema.ts`).
//!
//! ## Known divergences from strict schema
//!
//! - `JsonRpcRequest.id: Value` is permissive; schema declares `RequestId = string | number`.
//! - `JsonRpcResponse` combines success/error into one struct with `Option<result>`/`Option<error>`;
//!   schema declares separate `JSONRPCResultResponse` and `JSONRPCErrorResponse` joined by
//!   the `JSONRPCResponse` union. The current shape produces correct wire output via
//!   `skip_serializing_if = "Option::is_none"` but loses the schema's type-level guarantee
//!   that exactly one of `result`/`error` is present.
//! - `JsonRpcMessage` has an `Error` variant not in the schema union; harmless on the wire
//!   (untagged enum), unused in dispatch.
//! - `RequestParams._meta` is `Option<Meta>` (legacy 2025-11-25 shape); schema requires
//!   `_meta: RequestMetaObject` on every request.
//! - `Result.resultType` is not modeled on `ResultWithMeta` (the generic envelope);
//!   typed result structs carry it directly.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::meta::{MetaObject, ProgressToken, RequestMetaObject};
use crate::traits::{
    HasData, HasDataParam, HasMeta, HasMetaParam, HasProgressTokenParam, Params, RpcResult,
};

/// JSON-RPC version constant
pub const JSONRPC_VERSION: &str = "2.0";

/// JSON-RPC `params` object with optional `_meta` and method-specific arguments
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestParams {
    /// Schema-typed `_meta` per `RequestMetaObject`. Required fields like
    /// `io.modelcontextprotocol/protocolVersion`, `clientInfo`, and
    /// `clientCapabilities` live as named fields on the typed struct.
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "_meta")]
    pub meta: Option<RequestMetaObject>,

    /// All other method-specific parameters
    #[serde(flatten)]
    pub other: HashMap<String, Value>,
}

impl Params for RequestParams {}

impl HasMeta for RequestParams {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        // Surface the typed RequestMetaObject as a loose map for trait consumers.
        self.meta.as_ref().and_then(|m| {
            serde_json::to_value(m)
                .ok()
                .and_then(|v| v.as_object().map(|o| o.clone().into_iter().collect()))
        })
    }
}

impl HasProgressTokenParam for RequestParams {
    fn progress_token(&self) -> Option<&ProgressToken> {
        self.meta.as_ref()?.progress_token.as_ref()
    }
}

impl HasDataParam for RequestParams {
    fn data(&self) -> &HashMap<String, Value> {
        &self.other
    }
}

/// `params` shape for any request that extends `PaginatedRequest` —
/// `PaginatedRequestParams extends RequestParams { cursor?: Cursor }`.
///
/// Used directly by `ListResourcesRequest`, `ListResourceTemplatesRequest`,
/// `ListPromptsRequest`, `ListToolsRequest` — one struct, one wire shape.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedRequestParams {
    /// Opaque pagination cursor — server returns results after this point.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<crate::meta::Cursor>,

    /// Schema-typed `_meta` per `RequestMetaObject` (inherited via
    /// `PaginatedRequestParams extends RequestParams`).
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<RequestMetaObject>,
}

impl PaginatedRequestParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_cursor(mut self, cursor: crate::meta::Cursor) -> Self {
        self.cursor = Some(cursor);
        self
    }

    pub fn with_meta(mut self, meta: RequestMetaObject) -> Self {
        self.meta = Some(meta);
        self
    }
}

impl Params for PaginatedRequestParams {}

impl HasMetaParam for PaginatedRequestParams {
    fn meta(&self) -> Option<&HashMap<String, Value>> {
        // Surface only the namespaced `extra` keys from `RequestMetaObject`.
        // Structured spec fields (protocolVersion, clientInfo, clientCapabilities,
        // progressToken, logLevel) are accessed via `self.meta` directly.
        self.meta.as_ref().map(|m| &m.extra)
    }
}

impl HasMetaParam for RequestParams {
    fn meta(&self) -> Option<&HashMap<String, Value>> {
        // Surface the typed `RequestMetaObject`'s namespaced `extra` keys.
        // The structured spec fields (protocolVersion, clientInfo, clientCapabilities,
        // progressToken, logLevel) aren't reachable via this loose trait — callers
        // that need them must access `self.meta` directly.
        self.meta.as_ref().map(|m| &m.extra)
    }
}

/// Generic result envelope. Every `Result` extends `{_meta?: MetaObject}` —
/// loose key-value, namespaced per schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultWithMeta {
    /// The result data
    #[serde(flatten)]
    pub data: HashMap<String, Value>,

    /// Optional `_meta` — loose `MetaObject` per schema.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}

impl ResultWithMeta {
    pub fn new(data: HashMap<String, Value>) -> Self {
        Self { data, meta: None }
    }

    pub fn with_meta(mut self, meta: MetaObject) -> Self {
        self.meta = Some(meta);
        self
    }

    pub fn from_value(value: Value) -> Self {
        match value {
            Value::Object(map) => Self {
                data: map.into_iter().collect(),
                meta: None,
            },
            _ => Self {
                data: HashMap::new(),
                meta: None,
            },
        }
    }
}

impl HasData for ResultWithMeta {
    fn data(&self) -> HashMap<String, Value> {
        self.data.clone()
    }
}

impl HasMeta for ResultWithMeta {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        self.meta.clone()
    }
}

impl RpcResult for ResultWithMeta {}

/// A standard JSON-RPC 2.0 request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<RequestParams>,
}

impl JsonRpcRequest {
    pub fn new(id: Value, method: String) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            method,
            params: None,
        }
    }

    pub fn with_params(mut self, params: RequestParams) -> Self {
        self.params = Some(params);
        self
    }
}

/// A standard JSON-RPC 2.0 response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ResultWithMeta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    pub fn success(id: Value, result: ResultWithMeta) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Value, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// JSON-RPC 2.0 error object.
///
/// See [JSON-RPC 2.0 Error Object](https://www.jsonrpc.org/specification#error_object).
/// The MCP schema's `ParseError`, `InvalidRequestError`, `MethodNotFoundError`,
/// `InvalidParamsError`, and `InternalError` interfaces all share this shape;
/// the factories below mint each with the canonical error code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    pub fn new(code: i32, message: String) -> Self {
        Self {
            code,
            message,
            data: None,
        }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    // Standard JSON-RPC error codes
    pub fn parse_error() -> Self {
        Self::new(-32700, "Parse error".to_string())
    }

    pub fn invalid_request() -> Self {
        Self::new(-32600, "Invalid Request".to_string())
    }

    pub fn method_not_found() -> Self {
        Self::new(-32601, "Method not found".to_string())
    }

    pub fn invalid_params() -> Self {
        Self::new(-32602, "Invalid params".to_string())
    }

    pub fn internal_error() -> Self {
        Self::new(-32603, "Internal error".to_string())
    }
}

/// A JSON-RPC 2.0 notification (no response expected)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<RequestParams>,
}

impl JsonRpcNotification {
    pub fn new(method: String) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method,
            params: None,
        }
    }

    pub fn with_params(mut self, params: RequestParams) -> Self {
        self.params = Some(params);
        self
    }
}

/// Unified JSON-RPC message type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcMessage {
    Request(JsonRpcRequest),
    Response(JsonRpcResponse),
    Notification(JsonRpcNotification),
    Error(JsonRpcError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_request_params_with_meta() {
        // Schema-aligned: `RequestParams._meta` is a typed `RequestMetaObject`
        // with required namespaced fields. `cursor`/`total`/`hasMore` are NOT
        // inside `_meta` per schema — they belong on `PaginatedRequestParams`
        // (cursor) and `PaginatedResult` (total/hasMore).
        let meta = RequestMetaObject::new(
            "DRAFT-2026-v1",
            crate::initialize::Implementation::new("test-client", "1.0.0"),
            crate::initialize::ClientCapabilities::default(),
        )
        .with_progress_token("test-token")
        .with_extra("sessionId", json!("s-123"));

        let params = RequestParams {
            meta: Some(meta),
            other: {
                let mut map = HashMap::new();
                map.insert("name".to_string(), json!("test"));
                map
            },
        };

        let json_str = serde_json::to_string(&params).unwrap();
        // Required namespaced fields on the wire:
        assert!(json_str.contains("io.modelcontextprotocol/protocolVersion"));
        assert!(json_str.contains("DRAFT-2026-v1"));
        assert!(json_str.contains("io.modelcontextprotocol/clientInfo"));
        // Optional `progressToken` + extra `sessionId`:
        assert!(json_str.contains("progressToken"));
        assert!(json_str.contains("test-token"));
        assert!(json_str.contains("sessionId"));
        // Method-specific arg in flattened `other`:
        assert!(json_str.contains("name"));

        let parsed: RequestParams = serde_json::from_str(&json_str).unwrap();
        assert!(parsed.meta.is_some());
        assert_eq!(
            parsed
                .meta
                .as_ref()
                .unwrap()
                .progress_token
                .as_ref()
                .unwrap()
                .as_str(),
            "test-token"
        );
        assert_eq!(parsed.meta.as_ref().unwrap().protocol_version, "DRAFT-2026-v1");
    }

    #[test]
    fn test_result_with_meta() {
        let mut data = HashMap::new();
        data.insert("result".to_string(), json!("success"));

        let mut meta = HashMap::new();
        meta.insert("total".to_string(), json!(42));

        let result = ResultWithMeta::new(data).with_meta(meta);

        // Test traits
        assert!(result.data().contains_key("result"));
        assert!(result.meta().unwrap().contains_key("total"));

        // Test serialization
        let json_str = serde_json::to_string(&result).unwrap();
        assert!(json_str.contains("result"));
        assert!(json_str.contains("_meta"));
        assert!(json_str.contains("total"));
    }
}
