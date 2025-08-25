//! MCP Templates Protocol Types
//!
//! This module defines types for templates in MCP.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Template definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    /// Template name
    pub name: String,
    /// Template content
    pub content: String,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Template variables
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Vec<TemplateVariable>>,
    /// Optional annotations for the template
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Value>,
}

/// Template variable definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateVariable {
    /// Variable name
    pub name: String,
    /// Variable type
    #[serde(rename = "type")]
    pub var_type: String,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether the variable is required
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

/// Template rendering request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderTemplateRequest {
    /// Template name
    pub name: String,
    /// Variable values
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Value>,
}

/// Template rendering response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderTemplateResponse {
    /// Rendered content
    pub content: String,
}

impl Template {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            content: String::new(),
            description: None,
            variables: None,
            annotations: None,
        }
    }
    
    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_variables(mut self, variables: Vec<TemplateVariable>) -> Self {
        self.variables = Some(variables);
        self
    }

    pub fn with_annotations(mut self, annotations: Value) -> Self {
        self.annotations = Some(annotations);
        self
    }
}

/// Request for resources/templates/list (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResourceTemplatesRequest {
    /// Optional cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<crate::meta::Cursor>,
}

impl ListResourceTemplatesRequest {
    pub fn new() -> Self {
        Self {
            cursor: None,
        }
    }
    
    pub fn with_cursor(mut self, cursor: crate::meta::Cursor) -> Self {
        self.cursor = Some(cursor);
        self
    }
}

impl Default for ListResourceTemplatesRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Result for resources/templates/list (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResourceTemplatesResult {
    /// Available templates
    pub templates: Vec<Template>,
    /// Optional cursor for next page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<crate::meta::Cursor>,
    /// Meta information (follows MCP Result interface)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "_meta",
        rename = "_meta"
    )]
    pub meta: Option<std::collections::HashMap<String, Value>>,
}

impl ListResourceTemplatesResult {
    pub fn new(templates: Vec<Template>) -> Self {
        Self {
            templates,
            next_cursor: None,
            meta: None,
        }
    }
    
    pub fn with_next_cursor(mut self, cursor: crate::meta::Cursor) -> Self {
        self.next_cursor = Some(cursor);
        self
    }

    pub fn with_meta(mut self, meta: std::collections::HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

// Trait implementations
use crate::traits::{HasData, HasMeta, RpcResult, Params};

impl HasData for ListResourceTemplatesResult {
    fn data(&self) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        map.insert("templates".to_string(), serde_json::to_value(&self.templates).unwrap_or(Value::Null));
        if let Some(cursor) = &self.next_cursor {
            map.insert("nextCursor".to_string(), serde_json::to_value(cursor).unwrap_or(Value::Null));
        }
        map
    }
}

impl HasMeta for ListResourceTemplatesResult {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        self.meta.clone()
    }
}

impl RpcResult for ListResourceTemplatesResult {}

impl Params for ListResourceTemplatesRequest {}
impl Params for GetTemplateRequest {}
impl Params for RenderTemplateRequest {}

/// Request for templates/get
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTemplateRequest {
    /// Template name
    pub name: String,
    /// Variable values for rendering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

/// Response for templates/get
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTemplateResponse {
    /// Template description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Rendered template content
    pub content: String,
}