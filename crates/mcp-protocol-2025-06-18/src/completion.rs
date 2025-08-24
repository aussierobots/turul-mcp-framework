//! MCP Completion Protocol Types
//!
//! This module defines types for completion requests in MCP.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Completion request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteRequest {
    /// The completion argument to complete
    pub argument: CompleteArgument,
    /// Reference to the tool or resource being completed
    pub ref_value: Value,
}

/// Argument being completed
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteArgument {
    /// Name of the argument
    pub name: String,
    /// Current value being completed
    pub value: String,
}

/// Completion suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionSuggestion {
    /// The completion value
    pub value: String,
    /// Optional human-readable label
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional annotations for the completion suggestion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Value>,
}

/// Completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionResponse {
    /// List of completion suggestions
    pub completions: Vec<CompletionSuggestion>,
}

impl CompletionSuggestion {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: None,
            description: None,
            annotations: None,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_annotations(mut self, annotations: Value) -> Self {
        self.annotations = Some(annotations);
        self
    }
}

impl CompletionResponse {
    pub fn new(completions: Vec<CompletionSuggestion>) -> Self {
        Self { completions }
    }
}
