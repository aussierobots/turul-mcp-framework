//! MCP Roots Protocol Types
//!
//! # Deprecation status (DRAFT-2026-v1)
//!
//! Per SEP-2577, the entire Roots client capability (`roots/list` RPC, `Root`
//! type, `RootsCapabilities`) is **deprecated** in this revision. New
//! implementations SHOULD NOT adopt it. Earliest removal: first revision
//! released on or after **2027-07-28**.
//!
//! Replacement: pass directories or files via tool parameters, resource URIs,
//! or server configuration.
//!
//! Note: the `notifications/roots/list_changed` notification was REMOVED
//! entirely in DRAFT-2026-v1 (not just deprecated). Only the request/response
//! surface remains during the 12-month migration window.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Root directory entry.
///
/// **Deprecated** per SEP-2577 — see module-level docs.
#[deprecated(
    since = "0.4.0",
    note = "Deprecated per SEP-2577 (DRAFT-2026-v1). \
            Replacement: pass directories or files via tool parameters, resource URIs, or server configuration. \
            Earliest removal: first release on/after 2027-07-28."
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    /// URI of the root (must start with "file://" currently)
    pub uri: String,
    /// Optional human-readable name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Optional metadata
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

#[allow(deprecated)]
impl Root {
    pub fn new(uri: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            name: None,
            meta: None,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Validate that the URI follows MCP requirements
    pub fn validate(&self) -> Result<(), String> {
        if !self.uri.starts_with("file://") {
            return Err("Root URI must start with 'file://'".to_string());
        }
        Ok(())
    }
}

/// Complete roots/list request.
///
/// Schema: `ListRootsRequest { method: "roots/list"; params?: RequestParams; }`.
/// `params` is the standard [`crate::json_rpc::RequestParams`] when present,
/// or omitted entirely — the schema's `params?` is honored by serializing as
/// `None`.
///
/// **Deprecated** per SEP-2577 — see module-level docs.
#[deprecated(
    since = "0.4.0",
    note = "Deprecated per SEP-2577 (DRAFT-2026-v1). \
            Replacement: pass directories or files via tool parameters, resource URIs, or server configuration. \
            Earliest removal: first release on/after 2027-07-28."
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRootsRequest {
    /// Method name (always "roots/list")
    pub method: String,
    /// Optional standard request params.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<crate::json_rpc::RequestParams>,
}

/// Response for `roots/list` — `{ roots: Root[] }`.
///
/// **Deprecated** per SEP-2577 — see module-level docs.
#[deprecated(
    since = "0.4.0",
    note = "Deprecated per SEP-2577 (DRAFT-2026-v1). \
            Replacement: pass directories or files via tool parameters, resource URIs, or server configuration. \
            Earliest removal: first release on/after 2027-07-28."
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(deprecated)]
pub struct ListRootsResult {
    /// Available roots.
    pub roots: Vec<Root>,
}

#[allow(deprecated)]
impl Default for ListRootsRequest {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(deprecated)]
impl ListRootsRequest {
    /// Construct without params (paramsless `roots/list`).
    pub fn new() -> Self {
        Self {
            method: "roots/list".to_string(),
            params: None,
        }
    }

    /// Construct with explicit per-request meta.
    pub fn with_meta(meta: crate::meta::RequestMetaObject) -> Self {
        Self {
            method: "roots/list".to_string(),
            params: Some(crate::json_rpc::RequestParams::new(meta)),
        }
    }

    /// Attach a fully-constructed params struct.
    pub fn with_params(mut self, params: crate::json_rpc::RequestParams) -> Self {
        self.params = Some(params);
        self
    }
}

#[allow(deprecated)]
impl ListRootsResult {
    pub fn new(roots: Vec<Root>) -> Self {
        Self { roots }
    }
}

// Trait implementations for protocol compliance
use crate::traits::*;

#[allow(deprecated)]
impl HasMethod for ListRootsRequest {
    fn method(&self) -> &str {
        &self.method
    }
}

#[allow(deprecated)]
impl HasParams for ListRootsRequest {
    fn params(&self) -> Option<&dyn Params> {
        self.params.as_ref().map(|p| p as &dyn Params)
    }
}

// `ListRootsResult` does not implement `HasMeta`, `HasData`, or `RpcResult` —
// the schema defines it as `{ roots: Root[] }` only (no `_meta`, no `extends
// Result`), so the trait contract `RpcResult: HasMeta + HasData` doesn't fit.

// ===========================================
// === Fine-Grained Roots Traits ===
// ===========================================

/// Trait for root metadata (URI, name, path info)
#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::{ListRootsRequest, ListRootsResult, Root};
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn test_root_creation() {
        let mut root = Root::new("file:///home/user/project").with_name("My Project");

        let meta = HashMap::from([
            ("version".to_string(), json!("1.0")),
            ("type".to_string(), json!("workspace")),
        ]);
        root = root.with_meta(meta.clone());

        assert_eq!(root.uri, "file:///home/user/project");
        assert_eq!(root.name, Some("My Project".to_string()));
        assert_eq!(root.meta, Some(meta));
    }

    #[test]
    fn test_root_validation() {
        let valid_root = Root::new("file:///valid/path");
        assert!(valid_root.validate().is_ok());

        let invalid_root = Root::new("http://invalid/path");
        assert!(invalid_root.validate().is_err());
    }

    #[test]
    fn test_list_roots_request() {
        let request = ListRootsRequest::new();
        assert_eq!(request.method, "roots/list");
    }

    #[test]
    fn test_list_roots_result() {
        let roots = vec![
            Root::new("file:///path1").with_name("Root 1"),
            Root::new("file:///path2").with_name("Root 2"),
        ];

        let result = ListRootsResult::new(roots.clone());
        assert_eq!(result.roots.len(), 2);
        assert_eq!(result.roots[0].name, Some("Root 1".to_string()));
    }

    #[test]
    fn test_serialization() {
        let root = Root::new("file:///test/path").with_name("Test Root");
        let json = serde_json::to_string(&root).unwrap();
        assert!(json.contains("file:///test/path"));
        assert!(json.contains("Test Root"));

        let parsed: Root = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.uri, "file:///test/path");
        assert_eq!(parsed.name, Some("Test Root".to_string()));
    }

    #[test]
    fn test_list_roots_request_matches_typescript_spec() {
        // Schema-anchor: `ListRootsRequest { method: "roots/list"; params?: RequestParams }`.
        // When `params` is present it carries the standard `RequestParams._meta:
        // RequestMetaObject` (required, typed).
        let meta = crate::meta::RequestMetaObject::new(
            "DRAFT-2026-v1",
            crate::initialize::Implementation::new("c", "1"),
            crate::initialize::ClientCapabilities::default(),
        )
        .with_extra("requestId", json!("req-123"));

        let request = ListRootsRequest::with_meta(meta);

        let json_value = serde_json::to_value(&request).unwrap();

        assert_eq!(json_value["method"], "roots/list");
        assert!(json_value["params"].is_object());
        assert_eq!(
            json_value["params"]["_meta"]["io.modelcontextprotocol/protocolVersion"],
            "DRAFT-2026-v1"
        );
        assert_eq!(json_value["params"]["_meta"]["requestId"], "req-123");
    }

    #[test]
    fn test_list_roots_result_matches_typescript_spec() {
        // `ListRootsResult { roots: Root[] }` per schema.
        let roots = vec![
            Root::new("file:///path1").with_name("Root 1"),
            Root::new("file:///path2").with_name("Root 2"),
        ];

        let result = ListRootsResult::new(roots);
        let json_value = serde_json::to_value(&result).unwrap();

        assert!(json_value["roots"].is_array());
        assert_eq!(json_value["roots"].as_array().unwrap().len(), 2);
        assert_eq!(json_value["roots"][0]["uri"], "file:///path1");
        assert_eq!(json_value["roots"][0]["name"], "Root 1");
        // Schema declares only `roots` — assert nothing else on the wire.
        let obj = json_value.as_object().unwrap();
        assert!(!obj.contains_key("_meta"));
        assert!(!obj.contains_key("resultType"));
        assert_eq!(obj.len(), 1);
    }


    #[test]
    fn test_optional_params_serialization() {
        // Test that requests without _meta don't serialize params when None
        let request = ListRootsRequest::new();
        let json_value = serde_json::to_value(&request).unwrap();

        assert_eq!(json_value["method"], "roots/list");
        // params should be absent since it's None
        assert!(
            json_value["params"].is_null()
                || !json_value.as_object().unwrap().contains_key("params")
        );
    }
}
