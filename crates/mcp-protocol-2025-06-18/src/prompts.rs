//! MCP Prompts Protocol Types
//!
//! This module defines the types used for the MCP prompts functionality.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::meta::Cursor;

/// A prompt descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    /// Name identifier for the prompt
    pub name: String,
    /// Optional human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Arguments that the prompt accepts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<PromptArgument>>,
    /// Optional annotations for the prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Value>,
}

impl Prompt {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            arguments: None,
            annotations: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_arguments(mut self, arguments: Vec<PromptArgument>) -> Self {
        self.arguments = Some(arguments);
        self
    }

    pub fn with_annotations(mut self, annotations: Value) -> Self {
        self.annotations = Some(annotations);
        self
    }
}

/// Argument definition for prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptArgument {
    /// Name of the argument
    pub name: String,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether the argument is required
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

impl PromptArgument {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            required: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn required(mut self) -> Self {
        self.required = Some(true);
        self
    }

    pub fn optional(mut self) -> Self {
        self.required = Some(false);
        self
    }
}

/// Parameters for prompts/list request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPromptsParams {
    /// Optional cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,
    /// Meta information (optional _meta field inside params)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl ListPromptsParams {
    pub fn new() -> Self {
        Self { 
            cursor: None,
            meta: None,
        }
    }

    pub fn with_cursor(mut self, cursor: Cursor) -> Self {
        self.cursor = Some(cursor);
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

impl Default for ListPromptsParams {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete prompts/list request (matches TypeScript ListPromptsRequest interface)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPromptsRequest {
    /// Method name (always "prompts/list")
    pub method: String,
    /// Request parameters
    pub params: ListPromptsParams,
}

impl ListPromptsRequest {
    pub fn new() -> Self {
        Self {
            method: "prompts/list".to_string(),
            params: ListPromptsParams::new(),
        }
    }

    pub fn with_cursor(mut self, cursor: Cursor) -> Self {
        self.params = self.params.with_cursor(cursor);
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params = self.params.with_meta(meta);
        self
    }
}

/// Result for prompts/list (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPromptsResult {
    /// Available prompts
    pub prompts: Vec<Prompt>,
    /// Optional cursor for next page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Cursor>,
    /// Meta information (follows MCP Result interface)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "_meta",
        rename = "_meta"
    )]
    pub meta: Option<HashMap<String, Value>>,
}

impl ListPromptsResult {
    pub fn new(prompts: Vec<Prompt>) -> Self {
        Self {
            prompts,
            next_cursor: None,
            meta: None,
        }
    }

    pub fn with_next_cursor(mut self, cursor: Cursor) -> Self {
        self.next_cursor = Some(cursor);
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

/// Parameters for prompts/get request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPromptParams {
    /// Name of the prompt to get
    pub name: String,
    /// Arguments to pass to the prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, Value>>,
    /// Meta information (optional _meta field inside params)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl GetPromptParams {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arguments: None,
            meta: None,
        }
    }

    pub fn with_arguments(mut self, arguments: HashMap<String, Value>) -> Self {
        self.arguments = Some(arguments);
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

/// Complete prompts/get request (matches TypeScript GetPromptRequest interface)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPromptRequest {
    /// Method name (always "prompts/get")
    pub method: String,
    /// Request parameters
    pub params: GetPromptParams,
}

impl GetPromptRequest {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            method: "prompts/get".to_string(),
            params: GetPromptParams::new(name),
        }
    }

    pub fn with_arguments(mut self, arguments: HashMap<String, Value>) -> Self {
        self.params = self.params.with_arguments(arguments);
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params = self.params.with_meta(meta);
        self
    }
}

/// Message content for prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PromptMessage {
    /// Text message
    Text {
        text: String,
    },
    /// Image message
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
    /// Resource reference
    Resource {
        resource: Value,
    },
}

impl PromptMessage {
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text {
            text: content.into(),
        }
    }

    pub fn image(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self::Image {
            data: data.into(),
            mime_type: mime_type.into(),
        }
    }

    pub fn resource(resource: Value) -> Self {
        Self::Resource { resource }
    }
}

/// Result for prompts/get (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPromptResult {
    /// Optional description of the prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Messages that make up the prompt
    pub messages: Vec<PromptMessage>,
    /// Meta information (follows MCP Result interface)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "_meta",
        rename = "_meta"
    )]
    pub meta: Option<HashMap<String, Value>>,
}

impl GetPromptResult {
    pub fn new(messages: Vec<PromptMessage>) -> Self {
        Self {
            description: None,
            messages,
            meta: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

// Trait implementations for prompts

use crate::traits::*;

// Trait implementations for ListPromptsParams
impl Params for ListPromptsParams {}

impl HasListPromptsParams for ListPromptsParams {
    fn cursor(&self) -> Option<&Cursor> {
        self.cursor.as_ref()
    }
}

impl HasMetaParam for ListPromptsParams {
    fn meta(&self) -> Option<&HashMap<String, Value>> {
        self.meta.as_ref()
    }
}

// Trait implementations for ListPromptsRequest
impl HasMethod for ListPromptsRequest {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasParams for ListPromptsRequest {
    fn params(&self) -> Option<&dyn Params> {
        Some(&self.params)
    }
}

// Trait implementations for ListPromptsResult
impl HasData for ListPromptsResult {
    fn data(&self) -> HashMap<String, Value> {
        let mut data = HashMap::new();
        data.insert("prompts".to_string(), serde_json::to_value(&self.prompts).unwrap_or(Value::Null));
        if let Some(ref next_cursor) = self.next_cursor {
            data.insert("nextCursor".to_string(), Value::String(next_cursor.as_str().to_string()));
        }
        data
    }
}

impl HasMeta for ListPromptsResult {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        self.meta.clone()
    }
}

impl RpcResult for ListPromptsResult {}

impl crate::traits::ListPromptsResult for ListPromptsResult {
    fn prompts(&self) -> &Vec<Prompt> {
        &self.prompts
    }
    
    fn next_cursor(&self) -> Option<&Cursor> {
        self.next_cursor.as_ref()
    }
}

// Trait implementations for GetPromptParams
impl Params for GetPromptParams {}

impl HasGetPromptParams for GetPromptParams {
    fn name(&self) -> &String {
        &self.name
    }
    
    fn arguments(&self) -> Option<&HashMap<String, Value>> {
        self.arguments.as_ref()
    }
}

impl HasMetaParam for GetPromptParams {
    fn meta(&self) -> Option<&HashMap<String, Value>> {
        self.meta.as_ref()
    }
}

// Trait implementations for GetPromptRequest
impl HasMethod for GetPromptRequest {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasParams for GetPromptRequest {
    fn params(&self) -> Option<&dyn Params> {
        Some(&self.params)
    }
}

// Trait implementations for GetPromptResult
impl HasData for GetPromptResult {
    fn data(&self) -> HashMap<String, Value> {
        let mut data = HashMap::new();
        data.insert("messages".to_string(), serde_json::to_value(&self.messages).unwrap_or(Value::Null));
        if let Some(ref description) = self.description {
            data.insert("description".to_string(), Value::String(description.clone()));
        }
        data
    }
}

impl HasMeta for GetPromptResult {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        self.meta.clone()
    }
}

impl RpcResult for GetPromptResult {}

impl crate::traits::GetPromptResult for GetPromptResult {
    fn description(&self) -> Option<&String> {
        self.description.as_ref()
    }
    
    fn messages(&self) -> &Vec<PromptMessage> {
        &self.messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_prompt_creation() {
        let arg = PromptArgument::new("topic")
            .with_description("The topic to write about")
            .required();

        let prompt = Prompt::new("write_essay")
            .with_description("Write an essay about a topic")
            .with_arguments(vec![arg]);

        assert_eq!(prompt.name, "write_essay");
        assert!(prompt.description.is_some());
        assert!(prompt.arguments.is_some());
    }

    #[test]
    fn test_prompt_message() {
        let text_msg = PromptMessage::text("Hello, world!");
        let image_msg = PromptMessage::image("base64data", "image/png");
        let resource_msg = PromptMessage::resource(json!({"uri": "file:///test.txt"}));

        assert!(matches!(text_msg, PromptMessage::Text { .. }));
        assert!(matches!(image_msg, PromptMessage::Image { .. }));
        assert!(matches!(resource_msg, PromptMessage::Resource { .. }));
    }

    #[test]
    fn test_get_prompt_request() {
        let mut args = HashMap::new();
        args.insert("topic".to_string(), json!("AI Safety"));

        let request = GetPromptRequest::new("write_essay")
            .with_arguments(args);

        assert_eq!(request.params.name, "write_essay");
        assert!(request.params.arguments.is_some());
    }

    #[test]
    fn test_get_prompt_response() {
        let messages = vec![
            PromptMessage::text("Write an essay about: "),
            PromptMessage::text("AI Safety"),
        ];

        let response = GetPromptResult::new(messages)
            .with_description("Generated essay prompt");

        assert_eq!(response.messages.len(), 2);
        assert!(response.description.is_some());
    }

    #[test]
    fn test_serialization() {
        let prompt = Prompt::new("test_prompt")
            .with_description("A test prompt");

        let json = serde_json::to_string(&prompt).unwrap();
        assert!(json.contains("test_prompt"));
        assert!(json.contains("A test prompt"));

        let parsed: Prompt = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "test_prompt");
    }
}

// ===========================================
// === Fine-Grained Prompt Traits ===
// ===========================================

/// Trait for prompt metadata (name, description)
pub trait HasPromptMetadata {
    /// Name identifier for the prompt
    fn name(&self) -> &str;
    
    /// Optional human-readable description
    fn description(&self) -> Option<&str> {
        None
    }
    
    /// Optional title for display purposes
    fn title(&self) -> Option<&str> {
        None
    }
}

/// Trait for prompt arguments specification
pub trait HasPromptArguments {
    /// Arguments that the prompt accepts
    fn arguments(&self) -> Option<&Vec<PromptArgument>>;
    
    /// Check if an argument is required
    fn is_required_argument(&self, name: &str) -> bool {
        if let Some(args) = self.arguments() {
            args.iter().any(|arg| arg.name == name && arg.required.unwrap_or(false))
        } else {
            false
        }
    }
    
    /// Get argument by name
    fn get_argument(&self, name: &str) -> Option<&PromptArgument> {
        if let Some(args) = self.arguments() {
            args.iter().find(|arg| arg.name == name)
        } else {
            None
        }
    }
}

/// Trait for prompt message generation and templating
pub trait HasPromptMessages {
    /// Generate prompt messages based on provided arguments
    fn render_messages(&self, args: Option<&HashMap<String, Value>>) -> Result<Vec<PromptMessage>, String>;
    
    /// Optional: Get message templates before argument substitution
    fn message_templates(&self) -> Vec<String> {
        vec![]
    }
    
    /// Optional: Validate arguments before rendering
    fn validate_arguments(&self, _args: &HashMap<String, Value>) -> Result<(), String> {
        Ok(())
    }
}

/// Trait for prompt annotations and custom metadata
pub trait HasPromptAnnotations {
    /// Optional annotations for client hints
    fn annotations(&self) -> Option<&Value>;
    
    /// Prompt category for organization
    fn category(&self) -> Option<&str> {
        None
    }
    
    /// Tags for discovery and filtering
    fn tags(&self) -> Vec<&str> {
        vec![]
    }
    
    /// Usage examples for documentation
    fn examples(&self) -> Vec<String> {
        vec![]
    }
}

/// Composed prompt definition trait (automatically implemented via blanket impl)
pub trait PromptDefinition: 
    HasPromptMetadata + 
    HasPromptArguments + 
    HasPromptMessages + 
    HasPromptAnnotations 
{
    /// Convert this prompt definition to a protocol Prompt struct
    fn to_prompt(&self) -> Prompt {
        let mut prompt = Prompt::new(self.name());
        
        if let Some(description) = self.description() {
            prompt = prompt.with_description(description);
        }
        
        if let Some(arguments) = self.arguments() {
            prompt = prompt.with_arguments(arguments.clone());
        }
        
        if let Some(annotations) = self.annotations() {
            prompt = prompt.with_annotations(annotations.clone());
        }
        
        prompt
    }
}

// Blanket implementation: any type implementing the fine-grained traits automatically gets PromptDefinition
impl<T> PromptDefinition for T 
where 
    T: HasPromptMetadata + HasPromptArguments + HasPromptMessages + HasPromptAnnotations 
{}