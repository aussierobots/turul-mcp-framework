//! MCP Roots Protocol Types
//!
//! This module defines types for root directory listing in MCP.

use serde::{Deserialize, Serialize};

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

/// Response for roots/list
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRootsResponse {
    /// Available roots
    pub roots: Vec<Root>,
}

impl ListRootsResponse {
    pub fn new(roots: Vec<Root>) -> Self {
        Self { roots }
    }
}