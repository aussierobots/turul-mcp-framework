//! MCP Roots Protocol Types
//!
//! This module defines types for root directory listing in MCP.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Root directory entry (per MCP spec)
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

/// Parameters for roots/list request (per MCP spec - no params required but can have _meta)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRootsParams {
    /// Meta information (optional _meta field inside params)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

/// Complete roots/list request (matches TypeScript ListRootsRequest interface)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRootsRequest {
    /// Method name (always "roots/list")
    pub method: String,
    /// Optional parameters (can be None since no actual params needed, but _meta can be present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<ListRootsParams>,
}

/// Response for `roots/list` — `{ roots: Root[] }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRootsResult {
    /// Available roots.
    pub roots: Vec<Root>,
}

impl Default for ListRootsParams {
    fn default() -> Self {
        Self::new()
    }
}

impl ListRootsParams {
    pub fn new() -> Self {
        Self { meta: None }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

impl Default for ListRootsRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl ListRootsRequest {
    pub fn new() -> Self {
        Self {
            method: "roots/list".to_string(),
            params: None,
        }
    }

    /// Attach a fully-constructed params struct.
    pub fn with_params(mut self, params: ListRootsParams) -> Self {
        self.params = Some(params);
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params = Some(ListRootsParams::new().with_meta(meta));
        self
    }
}

impl ListRootsResult {
    pub fn new(roots: Vec<Root>) -> Self {
        Self { roots }
    }
}

// Trait implementations for protocol compliance
use crate::traits::*;

impl Params for ListRootsParams {}

impl HasMetaParam for ListRootsParams {
    fn meta(&self) -> Option<&HashMap<String, Value>> {
        self.meta.as_ref()
    }
}

impl HasMethod for ListRootsRequest {
    fn method(&self) -> &str {
        &self.method
    }
}

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
        // Test ListRootsRequest matches: { method: string, params?: { _meta?: {...} } }
        let mut meta = HashMap::new();
        meta.insert("requestId".to_string(), json!("req-123"));

        let request = ListRootsRequest::new().with_meta(meta);

        let json_value = serde_json::to_value(&request).unwrap();

        assert_eq!(json_value["method"], "roots/list");
        assert!(json_value["params"].is_object());
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
