//! MCP Completion Protocol Types
//!
//! This module defines types for completion requests in MCP.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Completion request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteParams {
    /// The completion argument to complete
    pub argument: CompleteArgument,
    /// Reference to the tool or resource being completed
    pub ref_value: Value,
    /// Meta information (optional _meta field inside params)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<std::collections::HashMap<String, Value>>,
}

impl CompleteParams {
    pub fn new(argument: CompleteArgument, ref_value: Value) -> Self {
        Self {
            argument,
            ref_value,
            meta: None,
        }
    }

    pub fn with_meta(mut self, meta: std::collections::HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

/// Complete completion/complete request (matches TypeScript CompleteRequest interface)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteRequest {
    /// Method name (always "completion/complete")
    pub method: String,
    /// Request parameters
    pub params: CompleteParams,
}

impl CompleteRequest {
    pub fn new(argument: CompleteArgument, ref_value: Value) -> Self {
        Self {
            method: "completion/complete".to_string(),
            params: CompleteParams::new(argument, ref_value),
        }
    }

    pub fn with_meta(mut self, meta: std::collections::HashMap<String, Value>) -> Self {
        self.params = self.params.with_meta(meta);
        self
    }
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
    /// Meta information (follows MCP Result interface)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "_meta",
        rename = "_meta"
    )]
    pub meta: Option<std::collections::HashMap<String, Value>>,
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
        Self { 
            completions,
            meta: None,
        }
    }

    pub fn with_meta(mut self, meta: std::collections::HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

// Trait implementations for completion

use crate::traits::*;
use std::collections::HashMap;

// Trait implementations for CompleteParams
impl Params for CompleteParams {}

impl HasCompleteParams for CompleteParams {
    fn reference(&self) -> &Value {
        &self.ref_value
    }
    
    fn argument(&self) -> &CompleteArgument {
        &self.argument
    }
    
    fn context(&self) -> Option<&Value> {
        None // CompleteParams doesn't have explicit context field
    }
}

impl HasMetaParam for CompleteParams {
    fn meta(&self) -> Option<&std::collections::HashMap<String, Value>> {
        self.meta.as_ref()
    }
}

// Trait implementations for CompleteRequest
impl HasMethod for CompleteRequest {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasParams for CompleteRequest {
    fn params(&self) -> Option<&dyn Params> {
        Some(&self.params)
    }
}

// Trait implementations for CompletionResponse
impl HasData for CompletionResponse {
    fn data(&self) -> HashMap<String, Value> {
        let mut data = HashMap::new();
        data.insert("completion".to_string(), serde_json::to_value(&self.completions).unwrap_or(Value::Null));
        data
    }
}

impl HasMeta for CompletionResponse {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        self.meta.clone()
    }
}

impl RpcResult for CompletionResponse {}

impl HasCompletionResult for CompletionResponse {
    fn completion(&self) -> &Value {
        // This is a design issue - trait expects &Value but we have Vec<CompletionSuggestion>
        // For now, return a static empty array - this needs proper design consideration
        static EMPTY: Value = Value::Array(vec![]);
        &EMPTY
    }
}

impl CompleteResult for CompletionResponse {}
