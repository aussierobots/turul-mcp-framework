//! MCP Completion Protocol Types
//!
//! This module defines types for completion requests in MCP.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A reference to a resource or resource template definition (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTemplateReference {
    #[serde(rename = "type")]
    pub ref_type: String, // "ref/resource"
    /// The URI or URI template of the resource
    #[serde(rename = "uri")]
    pub uri: String,
}

/// Identifies a prompt (per MCP spec) - extends BaseMetadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptReference {
    #[serde(rename = "type")]
    pub ref_type: String, // "ref/prompt"
    /// The name of the prompt
    pub name: String,
    /// Optional description from BaseMetadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Union type for completion references (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CompletionReference {
    ResourceTemplate(ResourceTemplateReference),
    Prompt(PromptReference),
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
    #[serde(rename = "ref")]
    pub reference: CompletionReference,
    /// The argument being completed
    pub argument: CompleteArgument,
    /// Optional context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<CompletionContext>,
    /// Meta information (optional _meta field inside params)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl CompleteParams {
    pub fn new(reference: CompletionReference, argument: CompleteArgument) -> Self {
        Self {
            reference,
            argument,
            context: None,
            meta: None,
        }
    }

    pub fn with_context(mut self, context: CompletionContext) -> Self {
        self.context = Some(context);
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
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

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params = self.params.with_meta(meta);
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
impl ResourceTemplateReference {
    pub fn new(uri: impl Into<String>) -> Self {
        Self {
            ref_type: "ref/resource".to_string(),
            uri: uri.into(),
        }
    }
}

impl PromptReference {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            ref_type: "ref/prompt".to_string(),
            name: name.into(),
            description: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

impl CompletionReference {
    pub fn resource(uri: impl Into<String>) -> Self {
        Self::ResourceTemplate(ResourceTemplateReference::new(uri))
    }

    pub fn prompt(name: impl Into<String>) -> Self {
        Self::Prompt(PromptReference::new(name))
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

impl Default for CompletionContext {
    fn default() -> Self {
        Self::new()
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
use crate::traits::*;

impl Params for CompleteParams {}

impl HasMetaParam for CompleteParams {
    fn meta(&self) -> Option<&HashMap<String, Value>> {
        self.meta.as_ref()
    }
}

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

impl HasData for CompleteResult {
    fn data(&self) -> HashMap<String, Value> {
        let mut data = HashMap::new();
        data.insert("completion".to_string(), serde_json::to_value(&self.completion).unwrap_or(Value::Null));
        data
    }
}

impl HasMeta for CompleteResult {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        self.meta.clone()
    }
}

impl RpcResult for CompleteResult {}

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
        let mut request = CompleteRequest::new(
            self.reference().clone(),
            self.argument().clone()
        );
        if let Some(context) = self.context() {
            request = request.with_context(context.clone());
        }
        request
    }
}

// Blanket implementation: any type implementing the fine-grained traits automatically gets CompletionDefinition
impl<T> CompletionDefinition for T 
where 
    T: HasCompletionMetadata + HasCompletionContext + HasCompletionHandling 
{}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_resource_template_reference() {
        let ref_obj = ResourceTemplateReference::new("file:///test/{name}.txt");
        
        assert_eq!(ref_obj.ref_type, "ref/resource");
        assert_eq!(ref_obj.uri, "file:///test/{name}.txt");
        
        let json_value = serde_json::to_value(&ref_obj).unwrap();
        assert_eq!(json_value["type"], "ref/resource");
        assert_eq!(json_value["uri"], "file:///test/{name}.txt");
    }

    #[test]
    fn test_prompt_reference() {
        let ref_obj = PromptReference::new("test_prompt")
            .with_description("A test prompt");
        
        assert_eq!(ref_obj.ref_type, "ref/prompt");
        assert_eq!(ref_obj.name, "test_prompt");
        assert_eq!(ref_obj.description, Some("A test prompt".to_string()));
        
        let json_value = serde_json::to_value(&ref_obj).unwrap();
        assert_eq!(json_value["type"], "ref/prompt");
        assert_eq!(json_value["name"], "test_prompt");
        assert_eq!(json_value["description"], "A test prompt");
    }

    #[test]
    fn test_completion_reference_union() {
        let resource_ref = CompletionReference::resource("file:///test.txt");
        let prompt_ref = CompletionReference::prompt("my_prompt");
        
        // Test serialization
        let resource_json = serde_json::to_value(&resource_ref).unwrap();
        let prompt_json = serde_json::to_value(&prompt_ref).unwrap();
        
        assert_eq!(resource_json["type"], "ref/resource");
        assert_eq!(resource_json["uri"], "file:///test.txt");
        
        assert_eq!(prompt_json["type"], "ref/prompt");
        assert_eq!(prompt_json["name"], "my_prompt");
    }

    #[test]
    fn test_complete_request_matches_typescript_spec() {
        // Test CompleteRequest matches: { method: string, params: { ref: ..., argument: ..., context?: ..., _meta?: ... } }
        let mut meta = HashMap::new();
        meta.insert("requestId".to_string(), json!("req-123"));
        
        let mut context_args = HashMap::new();
        context_args.insert("userId".to_string(), "123".to_string());
        
        let context = CompletionContext::new().with_arguments(context_args);
        
        let request = CompleteRequest::new(
            CompletionReference::prompt("test_prompt"),
            CompleteArgument::new("arg_name", "partial_value")
        )
        .with_context(context)
        .with_meta(meta);
        
        let json_value = serde_json::to_value(&request).unwrap();
        
        assert_eq!(json_value["method"], "completion/complete");
        assert!(json_value["params"].is_object());
        assert!(json_value["params"]["ref"].is_object());
        assert_eq!(json_value["params"]["ref"]["type"], "ref/prompt");
        assert_eq!(json_value["params"]["ref"]["name"], "test_prompt");
        assert_eq!(json_value["params"]["argument"]["name"], "arg_name");
        assert_eq!(json_value["params"]["argument"]["value"], "partial_value");
        assert!(json_value["params"]["context"].is_object());
        assert_eq!(json_value["params"]["context"]["arguments"]["userId"], "123");
        assert_eq!(json_value["params"]["_meta"]["requestId"], "req-123");
    }

    #[test]
    fn test_complete_result_matches_typescript_spec() {
        // Test CompleteResult matches: { completion: { values: string[], total?: number, hasMore?: boolean }, _meta?: ... }
        let mut meta = HashMap::new();
        meta.insert("executionTime".to_string(), json!(42));
        
        let completion = CompletionResult::new(vec![
            "option1".to_string(),
            "option2".to_string(),
            "option3".to_string()
        ])
        .with_total(100)
        .with_has_more(true);
        
        let result = CompleteResult::new(completion)
            .with_meta(meta);
        
        let json_value = serde_json::to_value(&result).unwrap();
        
        assert!(json_value["completion"].is_object());
        assert!(json_value["completion"]["values"].is_array());
        assert_eq!(json_value["completion"]["values"].as_array().unwrap().len(), 3);
        assert_eq!(json_value["completion"]["values"][0], "option1");
        assert_eq!(json_value["completion"]["total"], 100);
        assert_eq!(json_value["completion"]["hasMore"], true);
        assert_eq!(json_value["_meta"]["executionTime"], 42);
    }

    #[test]
    fn test_serialization() {
        let request = CompleteRequest::new(
            CompletionReference::resource("file:///test/{id}.txt"),
            CompleteArgument::new("id", "test")
        );
        
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("completion/complete"));
        assert!(json.contains("ref/resource"));
        assert!(json.contains("file:///test/{id}.txt"));
        
        let parsed: CompleteRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.method, "completion/complete");
    }
}
