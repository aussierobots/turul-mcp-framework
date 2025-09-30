//! MCP Ping Protocol Types
//!
//! This module defines types for the ping functionality in MCP.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Request for ping (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PingRequest {
    /// Method name (always "ping")
    pub method: String,
    /// No parameters for ping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl PingRequest {
    pub fn new() -> Self {
        Self {
            method: "ping".to_string(),
            params: None,
        }
    }
}

impl Default for PingRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Empty result for successful operations (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmptyResult {
    /// Meta information (follows MCP Result interface)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "_meta",
        rename = "_meta"
    )]
    pub meta: Option<HashMap<String, Value>>,
}

impl EmptyResult {
    pub fn new() -> Self {
        Self { meta: None }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

impl Default for EmptyResult {
    fn default() -> Self {
        Self::new()
    }
}

// Trait implementations for EmptyResult
use crate::traits::{HasData, HasMeta, RpcResult};

impl HasData for EmptyResult {
    fn data(&self) -> HashMap<String, Value> {
        HashMap::new()
    }
}

impl HasMeta for EmptyResult {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        self.meta.clone()
    }
}

impl RpcResult for EmptyResult {}

// Trait implementations for protocol compliance
use crate::traits::Params;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmptyParams;

impl Params for EmptyParams {}

// Note: PingRequest contains method field which is handled at the request level
// The actual ping params would be EmptyParams in the params field

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_ping_request() {
        let ping = PingRequest::new();
        assert_eq!(ping.method, "ping");
        assert!(ping.params.is_none());

        let json = serde_json::to_value(&ping).unwrap();
        assert_eq!(json["method"], "ping");
    }

    #[test]
    fn test_empty_result() {
        let result = EmptyResult::new();
        assert!(result.meta.is_none());

        let meta = HashMap::from([("test".to_string(), json!("value"))]);
        let result_with_meta = EmptyResult::new().with_meta(meta.clone());
        assert_eq!(result_with_meta.meta, Some(meta));
    }

    #[test]
    fn test_empty_result_serialization() {
        let result = EmptyResult::new();
        let json = serde_json::to_value(&result).unwrap();

        // Should serialize to empty object (no _meta field when None)
        assert_eq!(json, json!({}));

        let meta = HashMap::from([("progressToken".to_string(), json!("test-123"))]);
        let result_with_meta = EmptyResult::new().with_meta(meta);
        let json_with_meta = serde_json::to_value(&result_with_meta).unwrap();
        assert!(json_with_meta["_meta"].is_object());
    }
}
