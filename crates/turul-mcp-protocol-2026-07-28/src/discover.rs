//! `server/discover` types for MCP DRAFT-2026-v1.
//!
//! The stateless 2026 core replaces the 2025-11-25 `initialize`/`initialized`
//! handshake with two mechanisms:
//!
//! 1. **Per-request capability negotiation** via [`RequestMetaObject`]
//!    (`io.modelcontextprotocol/protocolVersion`, `clientInfo`, `clientCapabilities`).
//! 2. **On-demand server discovery** via `server/discover`. Servers MUST
//!    implement it; clients MAY call it but are not required to.
//!
//! [`RequestMetaObject`]: crate::meta::RequestMetaObject

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::initialize::{ClientCapabilities, Implementation, ServerCapabilities};
use crate::meta::MetaObject;
use crate::result_type::ResultType;

/// Wire method string for the discover RPC.
pub const SERVER_DISCOVER_METHOD: &str = "server/discover";

/// Client → server `server/discover` request.
///
/// `DiscoverRequest extends JSONRPCRequest { method: "server/discover", params: RequestParams }`.
///
/// The `jsonrpc: "2.0"` and `id` fields are supplied by the
/// [`JsonRpcRequest`](crate::JsonRpcRequest) envelope when this typed
/// payload is wrapped for the wire — matches existing crate convention
/// (`CallToolRequest`, `ListToolsRequest`, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverRequest {
    /// Always `"server/discover"`.
    pub method: String,

    /// Standard request params; carries `_meta: RequestMetaObject` in spec-strict
    /// usage. Today's [`RequestParams`](crate::json_rpc::RequestParams) keeps
    /// `_meta` optional transitionally.
    pub params: crate::json_rpc::RequestParams,
}

impl DiscoverRequest {
    /// Construct a discover request with the required `_meta` (per-request
    /// capability negotiation, mandatory in DRAFT-2026-v1 stateless core).
    pub fn new(meta: crate::meta::RequestMetaObject) -> Self {
        Self {
            method: SERVER_DISCOVER_METHOD.to_string(),
            params: crate::json_rpc::RequestParams::new(meta),
        }
    }

    /// Attach a fully-constructed params struct.
    pub fn with_params(mut self, params: crate::json_rpc::RequestParams) -> Self {
        self.params = params;
        self
    }
}

/// Server → client `server/discover` result.
///
/// Extends `CacheableResult`, hence carries the required `resultType`
/// discriminator (always [`ResultType::Complete`] for normal discovery
/// responses — InputRequired discovery is not a defined flow) plus the
/// `ttlMs`/`cacheScope` cache-control mixin (SEP-2549).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverResult {
    /// Discriminator — `"complete"`.
    #[serde(default)]
    pub result_type: ResultType,

    /// `CacheableResult.ttlMs` — required by schema (DiscoverResult extends CacheableResult).
    pub ttl_ms: u64,
    /// `CacheableResult.cacheScope` — required by schema.
    pub cache_scope: crate::caching::CacheScope,

    /// Protocol versions this server supports.
    /// The client should choose one of these in subsequent requests'
    /// `_meta.io.modelcontextprotocol/protocolVersion`.
    pub supported_versions: Vec<String>,

    /// Server capabilities.
    pub capabilities: ServerCapabilities,

    /// Server implementation info.
    pub server_info: Implementation,

    /// Optional natural-language guidance for LLMs using this server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,

    /// Optional `_meta` per `Result` schema — loose `MetaObject`.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}

impl DiscoverResult {
    /// Construct with the three required fields. Cache hint defaults to
    /// immediately-stale public (`ttlMs=0`, `cacheScope=public`);
    /// `instructions` and `_meta` start `None`.
    pub fn new(
        supported_versions: Vec<String>,
        capabilities: ServerCapabilities,
        server_info: Implementation,
    ) -> Self {
        Self {
            result_type: ResultType::Complete,
            ttl_ms: 0,
            cache_scope: crate::caching::CacheScope::Public,
            supported_versions,
            capabilities,
            server_info,
            instructions: None,
            meta: None,
        }
    }

    /// Set the cache-control hint (`ttlMs` + `cacheScope`).
    pub fn with_cache(mut self, ttl_ms: u64, cache_scope: crate::caching::CacheScope) -> Self {
        self.ttl_ms = ttl_ms;
        self.cache_scope = cache_scope;
        self
    }

    /// Attach natural-language guidance.
    pub fn with_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }
}

/// Envelope `JSONRPCResultResponse` specialized to carry a [`DiscoverResult`].
///
/// Matches the existing crate convention of pairing each `XResult` with an
/// `XResultResponse` wrapper for type-checked dispatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverResultResponse {
    /// Always `"2.0"`.
    pub jsonrpc: String,
    /// Request id this response is for.
    pub id: Value,
    pub result: DiscoverResult,
}

impl DiscoverResultResponse {
    pub fn new(id: Value, result: DiscoverResult) -> Self {
        Self {
            jsonrpc: crate::json_rpc::JSONRPC_VERSION.to_string(),
            id,
            result,
        }
    }
}

// Trait impls: `DiscoverRequest` satisfies `JsonRpcRequestTrait + DiscoverRequestTrait`.
impl crate::traits::HasMethod for DiscoverRequest {
    fn method(&self) -> &str {
        &self.method
    }
}
impl crate::traits::HasParams for DiscoverRequest {
    fn params(&self) -> Option<&dyn crate::traits::Params> {
        Some(&self.params as &dyn crate::traits::Params)
    }
}
impl crate::traits::RpcRequest for DiscoverRequest {}
impl crate::traits::DiscoverRequestTrait for DiscoverRequest {}

// `DiscoverResult` satisfies `RpcResult` and `HasMeta`.
impl crate::traits::HasMeta for DiscoverResult {
    fn meta(&self) -> Option<&MetaObject> {
        self.meta.as_ref()
    }
}
impl crate::traits::HasResultType for DiscoverResult {
    fn result_type(&self) -> ResultType {
        self.result_type.clone()
    }
}
impl crate::traits::RpcResult for DiscoverResult {}

// The client capabilities are *advertised* by the client per-request via
// `RequestMetaObject.client_capabilities`, not in the discover request itself.
// We re-export the type here for ergonomic discovery from `discover::`.
pub use crate::initialize::ClientCapabilities as ClientCapabilitiesRef;
#[allow(dead_code)]
const _: fn() = || {
    // Compile-time guard: ensure the ClientCapabilities type lives where we expect.
    let _: ClientCapabilities;
};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture_impl() -> Implementation {
        Implementation::new("test-server", "0.4.0")
    }

    fn fixture_caps() -> ServerCapabilities {
        ServerCapabilities::default()
    }

    fn fixture_meta() -> crate::meta::RequestMetaObject {
        crate::meta::RequestMetaObject::new(
            "DRAFT-2026-v1",
            Implementation::new("test-client", "1.0.0"),
            ClientCapabilities::default(),
        )
    }

    // --- DiscoverRequest ---

    #[test]
    fn discover_request_method_is_server_discover() {
        let r = DiscoverRequest::new(fixture_meta());
        assert_eq!(r.method, "server/discover");
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["method"], "server/discover");
    }

    #[test]
    fn discover_request_constant_matches_method() {
        assert_eq!(SERVER_DISCOVER_METHOD, "server/discover");
    }

    #[test]
    fn discover_request_has_params_object() {
        let r = DiscoverRequest::new(fixture_meta());
        let v = serde_json::to_value(&r).unwrap();
        assert!(v["params"].is_object(), "params must be present per DiscoverRequest schema");
    }

    #[test]
    fn discover_request_satisfies_rpc_trait() {
        // Generic function over the A8 trait abstraction.
        fn method_via_trait<R: crate::traits::DiscoverRequestTrait>(r: &R) -> &str {
            r.method_string()
        }
        let r = DiscoverRequest::new(fixture_meta());
        assert_eq!(method_via_trait(&r), "server/discover");
        assert_eq!(crate::traits::HasMethod::method(&r), "server/discover");
    }

    // --- DiscoverResult ---

    #[test]
    fn discover_result_serializes_required_fields() {
        let r = DiscoverResult::new(
            vec!["DRAFT-2026-v1".to_string(), "2025-11-25".to_string()],
            fixture_caps(),
            fixture_impl(),
        );
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["resultType"], "complete");
        assert!(v["supportedVersions"].is_array());
        assert_eq!(v["supportedVersions"][0], "DRAFT-2026-v1");
        assert_eq!(v["supportedVersions"][1], "2025-11-25");
        assert!(v["capabilities"].is_object());
        assert!(v["serverInfo"].is_object());
        assert_eq!(v["serverInfo"]["name"], "test-server");
    }

    #[test]
    fn discover_result_omits_optional_fields_when_none() {
        let r = DiscoverResult::new(
            vec!["DRAFT-2026-v1".to_string()],
            fixture_caps(),
            fixture_impl(),
        );
        let v = serde_json::to_value(&r).unwrap();
        assert!(
            !v.as_object().unwrap().contains_key("instructions"),
            "instructions omitted when None"
        );
        assert!(
            !v.as_object().unwrap().contains_key("_meta"),
            "_meta omitted when None"
        );
    }

    #[test]
    fn discover_result_serializes_instructions_when_present() {
        let r = DiscoverResult::new(
            vec!["DRAFT-2026-v1".to_string()],
            fixture_caps(),
            fixture_impl(),
        )
        .with_instructions("Use this server for testing only.");
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["instructions"], "Use this server for testing only.");
    }

    #[test]
    fn discover_result_round_trips() {
        let r = DiscoverResult::new(
            vec!["DRAFT-2026-v1".to_string()],
            fixture_caps(),
            fixture_impl(),
        )
        .with_instructions("hi");
        let v = serde_json::to_value(&r).unwrap();
        // CacheableResult mixin (DiscoverResult extends CacheableResult) — both
        // fields are required on the wire, camelCase.
        assert_eq!(v["ttlMs"], 0);
        assert_eq!(v["cacheScope"], "public");
        let parsed: DiscoverResult = serde_json::from_value(v).unwrap();
        assert_eq!(parsed.result_type, ResultType::Complete);
        assert_eq!(parsed.supported_versions, vec!["DRAFT-2026-v1".to_string()]);
        assert_eq!(parsed.instructions.as_deref(), Some("hi"));
        assert_eq!(parsed.cache_scope, crate::caching::CacheScope::Public);
    }

    #[test]
    fn discover_result_back_compat_accepts_missing_result_type() {
        // Per the `Result` schema, clients receiving a result without
        // `resultType` must treat it as "complete". Our serde default does this.
        let v = json!({
            "ttlMs": 0,
            "cacheScope": "public",
            "supportedVersions": ["2026-07-28"],
            "capabilities": {},
            "serverInfo": {"name": "s", "version": "0.4.0"}
        });
        let r: DiscoverResult = serde_json::from_value(v).unwrap();
        assert_eq!(r.result_type, ResultType::Complete);
    }

    // --- DiscoverResultResponse ---

    #[test]
    fn discover_result_response_wire_shape() {
        let r = DiscoverResult::new(
            vec!["DRAFT-2026-v1".to_string()],
            fixture_caps(),
            fixture_impl(),
        );
        let resp = DiscoverResultResponse::new(json!(1), r);
        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["jsonrpc"], "2.0");
        assert_eq!(v["id"], 1);
        assert!(v["result"].is_object());
        assert_eq!(v["result"]["resultType"], "complete");
        assert!(v["result"]["supportedVersions"].is_array());
    }
}
