//! MCP Roots Protocol Types
//!
//! This module defines types for root directory listing in MCP.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Root directory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    /// URI of the root
    pub uri: String,
    /// Optional human-readable name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Root {
    pub fn new(uri: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            name: None,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

/// Parameters for roots/list request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRootsParams {
    /// Meta information (optional _meta field inside params)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
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

impl Default for ListRootsParams {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete roots/list request (matches TypeScript ListRootsRequest interface)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRootsRequest {
    /// Method name (always "roots/list")
    pub method: String,
    /// Request parameters
    pub params: ListRootsParams,
}

impl ListRootsRequest {
    pub fn new() -> Self {
        Self {
            method: "roots/list".to_string(),
            params: ListRootsParams::new(),
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params = self.params.with_meta(meta);
        self
    }
}

/// Response for roots/list
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRootsResponse {
    /// Available roots
    pub roots: Vec<Root>,
    /// Meta information (follows MCP Result interface)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "_meta",
        rename = "_meta"
    )]
    pub meta: Option<HashMap<String, Value>>,
}

impl ListRootsResponse {
    pub fn new(roots: Vec<Root>) -> Self {
        Self { 
            roots,
            meta: None,
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

// Trait implementations for roots

use crate::traits::*;

// Trait implementations for ListRootsParams
impl Params for ListRootsParams {}

impl HasMetaParam for ListRootsParams {
    fn meta(&self) -> Option<&HashMap<String, Value>> {
        self.meta.as_ref()
    }
}

// Trait implementations for ListRootsRequest
impl HasMethod for ListRootsRequest {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasParams for ListRootsRequest {
    fn params(&self) -> Option<&dyn Params> {
        Some(&self.params)
    }
}

// Trait implementations for ListRootsResponse
impl HasData for ListRootsResponse {
    fn data(&self) -> HashMap<String, Value> {
        let mut data = HashMap::new();
        data.insert("roots".to_string(), serde_json::to_value(&self.roots).unwrap_or(Value::Null));
        data
    }
}

impl HasMeta for ListRootsResponse {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        self.meta.clone()
    }
}

impl RpcResult for ListRootsResponse {}

impl ListRootsResult for ListRootsResponse {
    fn roots(&self) -> &Vec<Root> {
        &self.roots
    }
}