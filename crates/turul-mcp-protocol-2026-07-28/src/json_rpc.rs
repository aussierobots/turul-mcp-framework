//! MCP-specific `params` and `result` shapes that layer on top of generic
//! JSON-RPC 2.0 wire envelopes from `turul-rpc`.
//!
//! This module **does not redefine** `JsonRpcRequest`, `JsonRpcResponse`,
//! `JsonRpcNotification`, `JsonRpcError`, `JsonRpcMessage`, or `RequestId` —
//! those come from [`turul_rpc`] and are re-exported from the crate root.
//! The schema's `JSONRPCResponse = JSONRPCResultResponse | JSONRPCErrorResponse`
//! union is `turul_rpc::JsonRpcResponse`; the success-only variant is
//! `turul_rpc::JsonRpcSuccessResponse`.
//!
//! What lives here is MCP layer:
//! - [`RequestParams`] — required `_meta: RequestMetaObject` per 2026-07-28.
//! - `NotificationParams` (in [`crate::notifications`]) — optional `_meta: MetaObject`.
//! - [`PaginatedRequestParams`] — `RequestParams + cursor?`.
//!
//! Typed result structs (`CallToolResult`, `ListToolsResult`, etc.) live in
//! their domain modules and implement [`crate::traits::RpcResult`] directly —
//! no generic envelope wrapper.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::meta::{ProgressToken, RequestMetaObject};
use crate::traits::{HasDataParam, HasMetaParam, HasProgressTokenParam, Params};

/// JSON-RPC version constant. Mirrors `turul_rpc::JSONRPC_VERSION` for the
/// few sites that still need a `&str` literal rather than the typed enum.
pub const JSONRPC_VERSION: &str = "2.0";

/// JSON-RPC `params` object for any request — `_meta` is **required** per
/// 2026-07-28 stateless core (carries `protocolVersion`, `clientInfo`,
/// `clientCapabilities` for per-request negotiation).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestParams {
    /// Schema-typed `_meta` per `RequestMetaObject`. Required fields like
    /// `io.modelcontextprotocol/protocolVersion`, `clientInfo`, and
    /// `clientCapabilities` live as named fields on the typed struct.
    #[serde(rename = "_meta")]
    pub meta: RequestMetaObject,

    /// All other method-specific parameters
    #[serde(flatten)]
    pub other: HashMap<String, Value>,
}

impl RequestParams {
    /// Construct with the required `_meta`. Extra parameters start empty.
    pub fn new(meta: RequestMetaObject) -> Self {
        Self {
            meta,
            other: HashMap::new(),
        }
    }

    /// Insert a method-specific parameter key.
    pub fn with(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.other.insert(key.into(), value.into());
        self
    }
}

impl Params for RequestParams {}

// `HasMeta` (returns `Option<&MetaObject>` — the loose `_meta?: MetaObject`
// shape used by Result and NotificationParams) intentionally NOT implemented
// for `RequestParams`. The schema's `RequestParams._meta: RequestMetaObject`
// is typed and REQUIRED — `request.meta` is the direct accessor; `HasMetaParam`
// exposes the namespaced `extra` keys for loose consumers.

impl HasProgressTokenParam for RequestParams {
    fn progress_token(&self) -> Option<&ProgressToken> {
        self.meta.progress_token.as_ref()
    }
}

impl HasDataParam for RequestParams {
    fn data(&self) -> &HashMap<String, Value> {
        &self.other
    }
}

// `NotificationParams` lives in `crate::notifications` (it predates this slice
// and has the same wire shape: `{ _meta?: MetaObject, [rest] }`). Framework
// trait impls are co-located with the struct there.

/// `params` shape for any request that extends `PaginatedRequest` —
/// `PaginatedRequestParams extends RequestParams { cursor?: Cursor }`.
///
/// Used directly by `ListResourcesRequest`, `ListResourceTemplatesRequest`,
/// `ListPromptsRequest`, `ListToolsRequest` — one struct, one wire shape.
/// `_meta` is **required** (inherited from `RequestParams`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedRequestParams {
    /// Opaque pagination cursor — server returns results after this point.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<crate::meta::Cursor>,

    /// Schema-typed `_meta` per `RequestMetaObject` (inherited via
    /// `PaginatedRequestParams extends RequestParams`). Required.
    #[serde(rename = "_meta")]
    pub meta: RequestMetaObject,
}

impl PaginatedRequestParams {
    /// Construct with the required `_meta`. No cursor by default.
    pub fn new(meta: RequestMetaObject) -> Self {
        Self { cursor: None, meta }
    }

    pub fn with_cursor(mut self, cursor: crate::meta::Cursor) -> Self {
        self.cursor = Some(cursor);
        self
    }

    pub fn with_meta(mut self, meta: RequestMetaObject) -> Self {
        self.meta = meta;
        self
    }
}

impl Params for PaginatedRequestParams {}

impl HasMetaParam for PaginatedRequestParams {
    fn meta(&self) -> Option<&HashMap<String, Value>> {
        // Surface only the namespaced `extra` keys from `RequestMetaObject`.
        // Structured spec fields (protocolVersion, clientInfo, clientCapabilities,
        // progressToken, logLevel) are accessed via `self.meta` directly.
        Some(&self.meta.extra)
    }
}

impl HasMetaParam for RequestParams {
    fn meta(&self) -> Option<&HashMap<String, Value>> {
        // Surface the typed `RequestMetaObject`'s namespaced `extra` keys.
        // The structured spec fields (protocolVersion, clientInfo, clientCapabilities,
        // progressToken, logLevel) aren't reachable via this loose trait — callers
        // that need them must access `self.meta` directly.
        Some(&self.meta.extra)
    }
}

// Wire-envelope types (`JsonRpcRequest`, `JsonRpcResponse` (the union),
// `JsonRpcSuccessResponse`, `JsonRpcNotification`, `JsonRpcError`,
// `JsonRpcMessage`, `RequestId`, `JsonRpcVersion`) come from `turul-rpc` and
// are re-exported from the crate root. The MCP types above (`RequestParams`,
// `NotificationParams`, `PaginatedRequestParams`) are what goes *inside* the
// envelope's `params` field per the schema's `Request.params: { [key: string]:
// any }`. Typed results (`CallToolResult`, `ListToolsResult`, etc.) go inside
// `result` directly — no generic envelope wrapper.

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
            "2026-07-28",
            crate::initialize::Implementation::new("test-client", "1.0.0"),
            crate::initialize::ClientCapabilities::default(),
        )
        .with_progress_token("test-token")
        .with_extra("sessionId", json!("s-123"));

        let params = RequestParams {
            meta,
            other: {
                let mut map = HashMap::new();
                map.insert("name".to_string(), json!("test"));
                map
            },
        };

        let json_str = serde_json::to_string(&params).unwrap();
        // Required namespaced fields on the wire:
        assert!(json_str.contains("io.modelcontextprotocol/protocolVersion"));
        assert!(json_str.contains("2026-07-28"));
        assert!(json_str.contains("io.modelcontextprotocol/clientInfo"));
        // Optional `progressToken` + extra `sessionId`:
        assert!(json_str.contains("progressToken"));
        assert!(json_str.contains("test-token"));
        assert!(json_str.contains("sessionId"));
        // Method-specific arg in flattened `other`:
        assert!(json_str.contains("name"));

        let parsed: RequestParams = serde_json::from_str(&json_str).unwrap();
        assert_eq!(
            parsed.meta.progress_token.as_ref().unwrap().as_str(),
            Some("test-token")
        );
        assert_eq!(parsed.meta.protocol_version, "2026-07-28");
    }

    #[test]
    fn test_request_params_rejects_missing_meta() {
        // Spec compliance: `_meta` is REQUIRED on every request per 2026-07-28.
        // Wire shape without `_meta` MUST fail to deserialize.
        let wire_without_meta = json!({"name": "test"});
        let r: Result<RequestParams, _> = serde_json::from_value(wire_without_meta);
        assert!(r.is_err(), "RequestParams without _meta must reject");
    }
}
