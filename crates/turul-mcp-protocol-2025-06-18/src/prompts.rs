//! MCP Prompts Protocol Types
//!
//! This module defines the types used for the MCP prompts functionality.

use crate::meta::Cursor;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Prompt annotations structure (matches TypeScript PromptAnnotations)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptAnnotations {
    /// Display name (precedence: Prompt.title > Prompt.name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    // Additional annotation fields can be added here as needed
}

impl Default for PromptAnnotations {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptAnnotations {
    pub fn new() -> Self {
        Self { title: None }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

/// A prompt descriptor (matches TypeScript Prompt interface exactly)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    /// Programmatic identifier (from BaseMetadata)
    pub name: String,
    /// Human-readable display name (from BaseMetadata)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Arguments that the prompt accepts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<PromptArgument>>,
    /// Optional MCP meta information
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl Prompt {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            title: None,
            description: None,
            arguments: None,
            meta: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_arguments(mut self, arguments: Vec<PromptArgument>) -> Self {
        self.arguments = Some(arguments);
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

/// The sender or recipient of messages and data in a conversation (matches MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
}

/// Argument definition for prompts (extends BaseMetadata per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptArgument {
    /// Name of the argument (from BaseMetadata)
    pub name: String,
    /// Human-readable display name (from BaseMetadata)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Human-readable description of the argument
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
            title: None,
            description: None,
            required: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
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

impl Default for ListPromptsParams {
    fn default() -> Self {
        Self::new()
    }
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

/// Complete prompts/list request (matches TypeScript ListPromptsRequest interface)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPromptsRequest {
    /// Method name (always "prompts/list")
    pub method: String,
    /// Request parameters
    pub params: ListPromptsParams,
}

impl Default for ListPromptsRequest {
    fn default() -> Self {
        Self::new()
    }
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

/// Parameters for prompts/get request (matches MCP GetPromptRequest.params exactly)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPromptParams {
    /// Name of the prompt to get
    pub name: String,
    /// Arguments to pass to the prompt (MCP spec: { [key: string]: string })
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, String>>,
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

    pub fn with_arguments(mut self, arguments: HashMap<String, String>) -> Self {
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

    pub fn with_arguments(mut self, arguments: HashMap<String, String>) -> Self {
        self.params = self.params.with_arguments(arguments);
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params = self.params.with_meta(meta);
        self
    }
}

/// Message content for prompts (matches MCP PromptMessage interface exactly)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptMessage {
    /// The role of the message sender
    pub role: Role,
    /// The content of the message (ContentBlock from MCP spec)
    pub content: ContentBlock,
}

/// Content block within a prompt message - now imports from content module
pub use crate::content::{ContentBlock, ResourceContents, ResourceReference};

impl PromptMessage {
    pub fn user_text(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: ContentBlock::text(content),
        }
    }

    pub fn assistant_text(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: ContentBlock::text(content),
        }
    }

    pub fn user_image(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: ContentBlock::image(data, mime_type),
        }
    }

    pub fn text(content: impl Into<String>) -> Self {
        // Backward compatibility - defaults to user
        Self::user_text(content)
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
        data.insert(
            "prompts".to_string(),
            serde_json::to_value(&self.prompts).unwrap_or(Value::Null),
        );
        if let Some(ref next_cursor) = self.next_cursor {
            data.insert(
                "nextCursor".to_string(),
                Value::String(next_cursor.as_str().to_string()),
            );
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

    fn arguments(&self) -> Option<&HashMap<String, String>> {
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
        data.insert(
            "messages".to_string(),
            serde_json::to_value(&self.messages).unwrap_or(Value::Null),
        );
        if let Some(ref description) = self.description {
            data.insert(
                "description".to_string(),
                Value::String(description.clone()),
            );
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
        let user_image_msg = PromptMessage::user_image("base64data", "image/png");
        let assistant_text_msg = PromptMessage::assistant_text("Response text");

        // Verify structure matches MCP spec: role + content
        assert_eq!(text_msg.role, Role::User);
        assert!(matches!(text_msg.content, ContentBlock::Text { .. }));

        assert_eq!(user_image_msg.role, Role::User);
        assert!(matches!(user_image_msg.content, ContentBlock::Image { .. }));

        assert_eq!(assistant_text_msg.role, Role::Assistant);
        assert!(matches!(
            assistant_text_msg.content,
            ContentBlock::Text { .. }
        ));
    }

    #[test]
    fn test_get_prompt_request() {
        let mut args = HashMap::new();
        args.insert("topic".to_string(), "AI Safety".to_string()); // Now uses String instead of Value

        let request = GetPromptRequest::new("write_essay").with_arguments(args);

        assert_eq!(request.params.name, "write_essay");
        assert!(request.params.arguments.is_some());

        // Verify arguments are string-to-string mapping per MCP spec
        if let Some(ref arguments) = request.params.arguments {
            assert_eq!(arguments.get("topic"), Some(&"AI Safety".to_string()));
        }
    }

    #[test]
    fn test_get_prompt_response() {
        let messages = vec![
            PromptMessage::user_text("Write an essay about: "),
            PromptMessage::assistant_text("AI Safety"),
        ];

        let response = GetPromptResult::new(messages).with_description("Generated essay prompt");

        assert_eq!(response.messages.len(), 2);
        assert!(response.description.is_some());

        // Verify messages have proper role structure per MCP spec
        assert_eq!(response.messages[0].role, Role::User);
        assert_eq!(response.messages[1].role, Role::Assistant);
    }

    #[test]
    fn test_serialization() {
        let prompt = Prompt::new("test_prompt").with_description("A test prompt");

        let json = serde_json::to_string(&prompt).unwrap();
        assert!(json.contains("test_prompt"));
        assert!(json.contains("A test prompt"));

        let parsed: Prompt = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "test_prompt");
    }
}
