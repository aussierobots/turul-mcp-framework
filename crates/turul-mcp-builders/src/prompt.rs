//! Prompt Builder for Runtime Prompt Construction
//!
//! This module provides a builder pattern for creating prompts at runtime
//! without requiring procedural macros. This enables dynamic prompt creation
//! for template-driven systems.

use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

// Import from protocol via alias
use turul_mcp_protocol::prompts::{
    ContentBlock, GetPromptResult, HasPromptAnnotations, HasPromptArguments, HasPromptDescription,
    HasPromptMeta, HasPromptMetadata, PromptArgument, PromptMessage,
};

/// Type alias for dynamic prompt generation function
pub type DynamicPromptFn = Box<
    dyn Fn(
            HashMap<String, String>,
        ) -> Pin<Box<dyn Future<Output = Result<GetPromptResult, String>> + Send>>
        + Send
        + Sync,
>;

/// Builder for creating prompts at runtime
pub struct PromptBuilder {
    name: String,
    title: Option<String>,
    description: Option<String>,
    arguments: Vec<PromptArgument>,
    messages: Vec<PromptMessage>,
    meta: Option<HashMap<String, Value>>,
    get_fn: Option<DynamicPromptFn>,
}

impl PromptBuilder {
    /// Create a new prompt builder with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            title: None,
            description: None,
            arguments: Vec::new(),
            messages: Vec::new(),
            meta: None,
            get_fn: None,
        }
    }

    /// Set the prompt title (display name)
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the prompt description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add an argument to the prompt
    pub fn argument(mut self, argument: PromptArgument) -> Self {
        self.arguments.push(argument);
        self
    }

    /// Add a string argument (required by default)
    pub fn string_argument(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        let arg = PromptArgument::new(name)
            .with_description(description)
            .required();
        self.arguments.push(arg);
        self
    }

    /// Add an optional string argument
    pub fn optional_string_argument(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        let arg = PromptArgument::new(name)
            .with_description(description)
            .optional();
        self.arguments.push(arg);
        self
    }

    /// Add a message to the prompt
    pub fn message(mut self, message: PromptMessage) -> Self {
        self.messages.push(message);
        self
    }

    /// Add a system message with text content
    pub fn system_message(mut self, text: impl Into<String>) -> Self {
        // Note: System role not defined in current MCP spec, using User as fallback
        self.messages
            .push(PromptMessage::user_text(format!("System: {}", text.into())));
        self
    }

    /// Add a user message with text content
    pub fn user_message(mut self, text: impl Into<String>) -> Self {
        self.messages.push(PromptMessage::user_text(text));
        self
    }

    /// Add an assistant message with text content
    pub fn assistant_message(mut self, text: impl Into<String>) -> Self {
        self.messages.push(PromptMessage::assistant_text(text));
        self
    }

    /// Add a user message with image content
    pub fn user_image(mut self, data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        self.messages
            .push(PromptMessage::user_image(data, mime_type));
        self
    }

    /// Add a templated user message (will be processed during get)
    pub fn template_user_message(mut self, template: impl Into<String>) -> Self {
        self.messages.push(PromptMessage::user_text(template));
        self
    }

    /// Add a templated assistant message (will be processed during get)
    pub fn template_assistant_message(mut self, template: impl Into<String>) -> Self {
        self.messages.push(PromptMessage::assistant_text(template));
        self
    }

    /// Set meta information
    pub fn meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Set the get function for dynamic prompt generation
    pub fn get<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(HashMap<String, String>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<GetPromptResult, String>> + Send + 'static,
    {
        self.get_fn = Some(Box::new(move |args| Box::pin(f(args))));
        self
    }

    /// Build the dynamic prompt
    pub fn build(self) -> Result<DynamicPrompt, String> {
        // If no get function provided, create a template processor
        let get_fn = if let Some(f) = self.get_fn {
            f
        } else {
            // Default template processor
            let messages = self.messages.clone();
            let description = self.description.clone();
            Box::new(move |args| {
                let messages = messages.clone();
                let description = description.clone();
                Box::pin(async move {
                    let processed_messages = process_template_messages(messages, &args)?;
                    let mut result = GetPromptResult::new(processed_messages);
                    if let Some(desc) = description {
                        result = result.with_description(desc);
                    }
                    Ok(result)
                })
                    as Pin<Box<dyn Future<Output = Result<GetPromptResult, String>> + Send>>
            })
        };

        Ok(DynamicPrompt {
            name: self.name,
            title: self.title,
            description: self.description,
            arguments: self.arguments,
            messages: self.messages,
            meta: self.meta,
            get_fn,
        })
    }
}

/// Dynamic prompt created by PromptBuilder
pub struct DynamicPrompt {
    name: String,
    title: Option<String>,
    description: Option<String>,
    arguments: Vec<PromptArgument>,
    #[allow(dead_code)]
    messages: Vec<PromptMessage>,
    meta: Option<HashMap<String, Value>>,
    get_fn: DynamicPromptFn,
}

impl DynamicPrompt {
    /// Get the prompt with argument substitution
    pub async fn get(&self, args: HashMap<String, String>) -> Result<GetPromptResult, String> {
        (self.get_fn)(args).await
    }
}

// Implement all fine-grained traits for DynamicPrompt
impl HasPromptMetadata for DynamicPrompt {
    fn name(&self) -> &str {
        &self.name
    }

    fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }
}

impl HasPromptDescription for DynamicPrompt {
    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

impl HasPromptArguments for DynamicPrompt {
    fn arguments(&self) -> Option<&Vec<PromptArgument>> {
        if self.arguments.is_empty() {
            None
        } else {
            Some(&self.arguments)
        }
    }
}

impl HasPromptAnnotations for DynamicPrompt {
    fn annotations(&self) -> Option<&turul_mcp_protocol::prompts::PromptAnnotations> {
        None // Annotations not currently implemented
    }
}

impl HasPromptMeta for DynamicPrompt {
    fn prompt_meta(&self) -> Option<&HashMap<String, Value>> {
        self.meta.as_ref()
    }
}

// PromptDefinition is automatically implemented via blanket impl!

/// Simple template processing for message content
fn process_template_messages(
    messages: Vec<PromptMessage>,
    args: &HashMap<String, String>,
) -> Result<Vec<PromptMessage>, String> {
    let mut processed = Vec::new();

    for message in messages {
        let processed_message = match message.content {
            ContentBlock::Text { text } => {
                let processed_text = process_template_string(&text, args);
                PromptMessage {
                    role: message.role,
                    content: ContentBlock::Text {
                        text: processed_text,
                    },
                }
            }
            // For other content types, just pass through unchanged
            other_content => PromptMessage {
                role: message.role,
                content: other_content,
            },
        };
        processed.push(processed_message);
    }

    Ok(processed)
}

/// Simple template string processing (replaces {arg_name} with values)
fn process_template_string(template: &str, args: &HashMap<String, String>) -> String {
    let mut result = template.to_string();

    for (key, value) in args {
        let placeholder = format!("{{{}}}", key);
        result = result.replace(&placeholder, value);
    }

    result
}

// Note: McpPrompt implementation will be provided by the turul-mcp-server crate
// since it depends on types from that crate (SessionContext, etc.)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_builder_basic() {
        let prompt = PromptBuilder::new("greeting_prompt")
            .title("Greeting Generator")
            .description("Generate personalized greetings")
            .string_argument("name", "The person's name")
            .user_message("Hello {name}! How are you today?")
            .build()
            .expect("Failed to build prompt");

        assert_eq!(prompt.name(), "greeting_prompt");
        assert_eq!(prompt.title(), Some("Greeting Generator"));
        assert_eq!(
            prompt.description(),
            Some("Generate personalized greetings")
        );
        assert_eq!(prompt.arguments().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_prompt_builder_template_processing() {
        let prompt = PromptBuilder::new("conversation_starter")
            .description("Start a conversation with someone")
            .string_argument("name", "Person's name")
            .optional_string_argument("topic", "Optional conversation topic")
            .user_message("Hi {name}! Nice to meet you.")
            .template_assistant_message("Hello! What would you like to talk about?")
            .user_message("Let's discuss {topic}")
            .build()
            .expect("Failed to build prompt");

        let mut args = HashMap::new();
        args.insert("name".to_string(), "Alice".to_string());
        args.insert("topic".to_string(), "Rust programming".to_string());

        let result = prompt.get(args).await.expect("Failed to get prompt");

        assert_eq!(result.messages.len(), 3);

        // Check template substitution
        if let ContentBlock::Text { text } = &result.messages[0].content {
            assert_eq!(text, "Hi Alice! Nice to meet you.");
        } else {
            panic!("Expected text content");
        }

        if let ContentBlock::Text { text } = &result.messages[2].content {
            assert_eq!(text, "Let's discuss Rust programming");
        } else {
            panic!("Expected text content");
        }
    }

    #[tokio::test]
    async fn test_prompt_builder_custom_get_function() {
        let prompt = PromptBuilder::new("dynamic_prompt")
            .description("Dynamic prompt with custom logic")
            .string_argument("mood", "Current mood")
            .get(|args| async move {
                let default_mood = "neutral".to_string();
                let mood = args.get("mood").unwrap_or(&default_mood);
                let message_text = match mood.as_str() {
                    "happy" => "That's wonderful! Tell me more about what's making you happy.",
                    "sad" => "I'm sorry to hear that. Would you like to talk about it?",
                    _ => "How are you feeling today?",
                };

                let messages = vec![
                    PromptMessage::user_text(format!("I'm feeling {}", mood)),
                    PromptMessage::assistant_text(message_text),
                ];

                Ok(GetPromptResult::new(messages).with_description("Mood-based conversation"))
            })
            .build()
            .expect("Failed to build prompt");

        let mut args = HashMap::new();
        args.insert("mood".to_string(), "happy".to_string());

        let result = prompt.get(args).await.expect("Failed to get prompt");

        assert_eq!(result.messages.len(), 2);
        assert_eq!(
            result.description,
            Some("Mood-based conversation".to_string())
        );

        if let ContentBlock::Text { text } = &result.messages[1].content {
            assert!(text.contains("wonderful"));
        } else {
            panic!("Expected text content");
        }
    }

    #[test]
    fn test_prompt_builder_arguments() {
        let prompt = PromptBuilder::new("complex_prompt")
            .string_argument("required_arg", "This is required")
            .optional_string_argument("optional_arg", "This is optional")
            .argument(
                PromptArgument::new("custom_arg")
                    .with_title("Custom Argument")
                    .with_description("A custom argument")
                    .required(),
            )
            .build()
            .expect("Failed to build prompt");

        let args = prompt.arguments().unwrap();
        assert_eq!(args.len(), 3);
        assert_eq!(args[0].name, "required_arg");
        assert_eq!(args[0].required, Some(true));
        assert_eq!(args[1].name, "optional_arg");
        assert_eq!(args[1].required, Some(false));
        assert_eq!(args[2].name, "custom_arg");
        assert_eq!(args[2].title, Some("Custom Argument".to_string()));
    }

    #[test]
    fn test_template_string_processing() {
        let template = "Hello {name}, welcome to {place}!";
        let mut args = HashMap::new();
        args.insert("name".to_string(), "Alice".to_string());
        args.insert("place".to_string(), "Wonderland".to_string());

        let result = process_template_string(template, &args);
        assert_eq!(result, "Hello Alice, welcome to Wonderland!");
    }
}
