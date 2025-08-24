//! MCP Prompts Protocol Types
//!
//! This module defines the types used for the MCP prompts functionality.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::tools::Cursor;

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
pub struct ListPromptsRequest {
    /// Optional cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,
}

impl ListPromptsRequest {
    pub fn new() -> Self {
        Self { cursor: None }
    }

    pub fn with_cursor(mut self, cursor: Cursor) -> Self {
        self.cursor = Some(cursor);
        self
    }
}

impl Default for ListPromptsRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Response for prompts/list
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPromptsResponse {
    /// Available prompts
    pub prompts: Vec<Prompt>,
    /// Optional cursor for next page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Cursor>,
}

impl ListPromptsResponse {
    pub fn new(prompts: Vec<Prompt>) -> Self {
        Self {
            prompts,
            next_cursor: None,
        }
    }

    pub fn with_next_cursor(mut self, cursor: Cursor) -> Self {
        self.next_cursor = Some(cursor);
        self
    }
}

/// Parameters for prompts/get request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPromptRequest {
    /// Name of the prompt to get
    pub name: String,
    /// Arguments to pass to the prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, Value>>,
}

impl GetPromptRequest {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arguments: None,
        }
    }

    pub fn with_arguments(mut self, arguments: HashMap<String, Value>) -> Self {
        self.arguments = Some(arguments);
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

/// Response for prompts/get
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPromptResponse {
    /// Optional description of the prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Messages that make up the prompt
    pub messages: Vec<PromptMessage>,
}

impl GetPromptResponse {
    pub fn new(messages: Vec<PromptMessage>) -> Self {
        Self {
            description: None,
            messages,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
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

        assert_eq!(request.name, "write_essay");
        assert!(request.arguments.is_some());
    }

    #[test]
    fn test_get_prompt_response() {
        let messages = vec![
            PromptMessage::text("Write an essay about: "),
            PromptMessage::text("AI Safety"),
        ];

        let response = GetPromptResponse::new(messages)
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