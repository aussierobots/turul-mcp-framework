//! MCP Templates Protocol Types
//!
//! This module defines types for templates in MCP.

use serde::{Deserialize, Serialize};
use serde_json::Value;

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
}

/// Request for templates/list
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTemplatesRequest {
    /// Optional cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<crate::tools::Cursor>,
}

/// Response for templates/list
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTemplatesResponse {
    /// Available templates
    pub templates: Vec<Template>,
    /// Optional cursor for next page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<crate::tools::Cursor>,
}

impl ListTemplatesResponse {
    pub fn new(templates: Vec<Template>) -> Self {
        Self {
            templates,
            next_cursor: None,
        }
    }
    
    pub fn with_next_cursor(mut self, cursor: crate::tools::Cursor) -> Self {
        self.next_cursor = Some(cursor);
        self
    }
}

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