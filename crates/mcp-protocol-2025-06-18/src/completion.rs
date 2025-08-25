//! MCP Completion Protocol Types
//!
//! This module defines types for completion requests in MCP.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Reference types for completion (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CompletionReference {
    #[serde(rename = "ref/resource")]
    Resource {
        uri: String,
    },
    #[serde(rename = "ref/prompt")]
    Prompt {
        name: String,
    },
}

/// Completion context (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionContext {
    /// Arguments context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, String>>,
}

/// Argument being completed (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteArgument {
    /// Name of the argument
    pub name: String,
    /// Current value being completed
    pub value: String,
}

/// Completion request parameters (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteParams {
    /// Reference to the prompt or resource being completed
    pub r#ref: CompletionReference,
    /// The argument being completed
    pub argument: CompleteArgument,
    /// Optional context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<CompletionContext>,
}

impl CompleteParams {
    pub fn new(reference: CompletionReference, argument: CompleteArgument) -> Self {
        Self {
            r#ref: reference,
            argument,
            context: None,
        }
    }

    pub fn with_context(mut self, context: CompletionContext) -> Self {
        self.context = Some(context);
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
    pub fn new(reference: CompletionReference, argument: CompleteArgument) -> Self {
        Self {
            method: "completion/complete".to_string(),
            params: CompleteParams::new(reference, argument),
        }
    }

    pub fn with_context(mut self, context: CompletionContext) -> Self {
        self.params = self.params.with_context(context);
        self
    }
}


/// Completion result (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionResult {
    /// The completion values
    pub values: Vec<String>,
    /// Optional total number of possible completions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u32>,
    /// Whether there are more completions available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_more: Option<bool>,
}

/// Complete completion/complete response (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteResult {
    /// The completion result
    pub completion: CompletionResult,
    /// Meta information (follows MCP Result interface)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "_meta",
        rename = "_meta"
    )]
    pub meta: Option<HashMap<String, Value>>,
}

impl CompletionResult {
    pub fn new(values: Vec<String>) -> Self {
        Self {
            values,
            total: None,
            has_more: None,
        }
    }

    pub fn with_total(mut self, total: u32) -> Self {
        self.total = Some(total);
        self
    }

    pub fn with_has_more(mut self, has_more: bool) -> Self {
        self.has_more = Some(has_more);
        self
    }
}

impl CompleteResult {
    pub fn new(completion: CompletionResult) -> Self {
        Self {
            completion,
            meta: None,
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

/// Convenience constructors
impl CompletionReference {
    pub fn resource(uri: impl Into<String>) -> Self {
        Self::Resource {
            uri: uri.into(),
        }
    }

    pub fn prompt(name: impl Into<String>) -> Self {
        Self::Prompt {
            name: name.into(),
        }
    }
}

impl CompleteArgument {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

impl CompletionContext {
    pub fn new() -> Self {
        Self {
            arguments: None,
        }
    }

    pub fn with_arguments(mut self, arguments: HashMap<String, String>) -> Self {
        self.arguments = Some(arguments);
        self
    }
}

// Trait implementations for protocol compliance
use crate::traits::Params;
impl Params for CompleteParams {}

// ===========================================
// === Fine-Grained Completion Traits ===
// ===========================================

/// Trait for completion metadata (method, reference type)
pub trait HasCompletionMetadata {
    /// The completion method name
    fn method(&self) -> &str;
    
    /// The reference being completed (prompt or resource)
    fn reference(&self) -> &CompletionReference;
}

/// Trait for completion context (argument, context)
pub trait HasCompletionContext {
    /// The argument being completed
    fn argument(&self) -> &CompleteArgument;
    
    /// Optional completion context
    fn context(&self) -> Option<&CompletionContext> {
        None
    }
}

/// Trait for completion validation and processing
pub trait HasCompletionHandling {
    /// Validate the completion request
    fn validate_request(&self, _request: &CompleteRequest) -> Result<(), String> {
        Ok(())
    }
    
    /// Filter completion values based on current input
    fn filter_completions(&self, values: Vec<String>, current_value: &str) -> Vec<String> {
        // Default: simple prefix matching
        values
            .into_iter()
            .filter(|v| v.to_lowercase().starts_with(&current_value.to_lowercase()))
            .collect()
    }
}

/// Composed completion definition trait (automatically implemented via blanket impl)
pub trait CompletionDefinition: 
    HasCompletionMetadata + 
    HasCompletionContext + 
    HasCompletionHandling 
{
    /// Convert this completion definition to a protocol CompleteRequest
    fn to_complete_request(&self) -> CompleteRequest {
        CompleteRequest::new(
            self.reference().clone(),
            self.argument().clone()
        )
        .with_context(self.context().cloned().unwrap_or_else(CompletionContext::new))
    }
}

// Blanket implementation: any type implementing the fine-grained traits automatically gets CompletionDefinition
impl<T> CompletionDefinition for T 
where 
    T: HasCompletionMetadata + HasCompletionContext + HasCompletionHandling 
{}
