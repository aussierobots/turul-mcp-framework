//! Message Builder for Runtime Sampling Message Construction
//!
//! This module provides a builder pattern for creating sampling messages and requests
//! at runtime. This enables dynamic message composition for LLM sampling operations.

use serde_json::Value;
use std::collections::HashMap;

// Import from protocol via alias
use turul_mcp_protocol::prompts::ContentBlock;
use turul_mcp_protocol::sampling::{
    CreateMessageParams, CreateMessageRequest, ModelHint, ModelPreferences, Role, SamplingMessage,
};

/// Builder for creating sampling messages and requests at runtime
pub struct MessageBuilder {
    messages: Vec<SamplingMessage>,
    model_preferences: Option<ModelPreferences>,
    system_prompt: Option<String>,
    include_context: Option<String>,
    temperature: Option<f64>,
    max_tokens: u32,
    stop_sequences: Option<Vec<String>>,
    metadata: Option<Value>,
    meta: Option<HashMap<String, Value>>,
}

impl MessageBuilder {
    /// Create a new message builder with default max_tokens
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            model_preferences: None,
            system_prompt: None,
            include_context: None,
            temperature: None,
            max_tokens: 1000, // Reasonable default
            stop_sequences: None,
            metadata: None,
            meta: None,
        }
    }

    /// Set maximum tokens for response generation
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Add a message to the conversation
    pub fn message(mut self, message: SamplingMessage) -> Self {
        self.messages.push(message);
        self
    }

    /// Add a system message with text content
    pub fn system(mut self, content: impl Into<String>) -> Self {
        self.messages.push(SamplingMessage {
            role: Role::System,
            content: ContentBlock::Text {
                text: content.into(),
            },
        });
        self
    }

    /// Add a user message with text content
    pub fn user_text(mut self, content: impl Into<String>) -> Self {
        self.messages.push(SamplingMessage {
            role: Role::User,
            content: ContentBlock::Text {
                text: content.into(),
            },
        });
        self
    }

    /// Add a user message with image content
    pub fn user_image(mut self, data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        self.messages.push(SamplingMessage {
            role: Role::User,
            content: ContentBlock::Image {
                data: data.into(),
                mime_type: mime_type.into(),
            },
        });
        self
    }

    /// Add an assistant message with text content
    pub fn assistant_text(mut self, content: impl Into<String>) -> Self {
        self.messages.push(SamplingMessage {
            role: Role::Assistant,
            content: ContentBlock::Text {
                text: content.into(),
            },
        });
        self
    }

    /// Set model preferences
    pub fn model_preferences(mut self, preferences: ModelPreferences) -> Self {
        self.model_preferences = Some(preferences);
        self
    }

    /// Create and set model preferences with builder pattern
    pub fn with_model_preferences<F>(mut self, f: F) -> Self
    where
        F: FnOnce(ModelPreferencesBuilder) -> ModelPreferencesBuilder,
    {
        let builder = ModelPreferencesBuilder::new();
        let preferences = f(builder).build();
        self.model_preferences = Some(preferences);
        self
    }

    /// Set system prompt
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set include context
    pub fn include_context(mut self, context: impl Into<String>) -> Self {
        self.include_context = Some(context.into());
        self
    }

    /// Set temperature for sampling (0.0-1.0)
    pub fn temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature.clamp(0.0, 2.0));
        self
    }

    /// Set stop sequences
    pub fn stop_sequences(mut self, sequences: Vec<String>) -> Self {
        self.stop_sequences = Some(sequences);
        self
    }

    /// Add a stop sequence
    pub fn stop_sequence(mut self, sequence: impl Into<String>) -> Self {
        if let Some(ref mut sequences) = self.stop_sequences {
            sequences.push(sequence.into());
        } else {
            self.stop_sequences = Some(vec![sequence.into()]);
        }
        self
    }

    /// Set metadata
    pub fn metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Set meta information
    pub fn meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Build a CreateMessageParams
    pub fn build_params(self) -> CreateMessageParams {
        let mut params = CreateMessageParams::new(self.messages, self.max_tokens);

        if let Some(preferences) = self.model_preferences {
            params = params.with_model_preferences(preferences);
        }
        if let Some(prompt) = self.system_prompt {
            params = params.with_system_prompt(prompt);
        }
        if let Some(temp) = self.temperature {
            params = params.with_temperature(temp);
        }
        if let Some(sequences) = self.stop_sequences {
            params = params.with_stop_sequences(sequences);
        }
        if let Some(meta) = self.meta {
            params = params.with_meta(meta);
        }

        // Set additional fields not in the with_* methods
        params.include_context = self.include_context;
        params.metadata = self.metadata;

        params
    }

    /// Build a complete CreateMessageRequest
    pub fn build_request(self) -> CreateMessageRequest {
        CreateMessageRequest {
            method: "sampling/createMessage".to_string(),
            params: self.build_params(),
        }
    }
}

impl Default for MessageBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for ModelPreferences
pub struct ModelPreferencesBuilder {
    hints: Vec<ModelHint>,
    cost_priority: Option<f64>,
    speed_priority: Option<f64>,
    intelligence_priority: Option<f64>,
}

impl ModelPreferencesBuilder {
    pub fn new() -> Self {
        Self {
            hints: Vec::new(),
            cost_priority: None,
            speed_priority: None,
            intelligence_priority: None,
        }
    }

    /// Add a model hint
    pub fn hint(mut self, hint: ModelHint) -> Self {
        self.hints.push(hint);
        self
    }

    /// Add Claude 3.5 Sonnet as preferred model
    pub fn prefer_claude_sonnet(self) -> Self {
        self.hint(ModelHint::Claude35Sonnet20241022)
    }

    /// Add Claude 3.5 Haiku as preferred model
    pub fn prefer_claude_haiku(self) -> Self {
        self.hint(ModelHint::Claude35Haiku20241022)
    }

    /// Add GPT-4o as preferred model
    pub fn prefer_gpt4o(self) -> Self {
        self.hint(ModelHint::Gpt4o)
    }

    /// Add GPT-4o Mini as preferred model
    pub fn prefer_gpt4o_mini(self) -> Self {
        self.hint(ModelHint::Gpt4oMini)
    }

    /// Prefer fast models
    pub fn prefer_fast(self) -> Self {
        self.hint(ModelHint::Claude35Haiku20241022)
            .hint(ModelHint::Gpt4oMini)
    }

    /// Prefer high-quality models
    pub fn prefer_quality(self) -> Self {
        self.hint(ModelHint::Claude35Sonnet20241022)
            .hint(ModelHint::Gpt4o)
    }

    /// Set cost priority (0.0-1.0)
    pub fn cost_priority(mut self, priority: f64) -> Self {
        self.cost_priority = Some(priority.clamp(0.0, 1.0));
        self
    }

    /// Set speed priority (0.0-1.0)
    pub fn speed_priority(mut self, priority: f64) -> Self {
        self.speed_priority = Some(priority.clamp(0.0, 1.0));
        self
    }

    /// Set intelligence priority (0.0-1.0)
    pub fn intelligence_priority(mut self, priority: f64) -> Self {
        self.intelligence_priority = Some(priority.clamp(0.0, 1.0));
        self
    }

    /// Build the ModelPreferences
    pub fn build(self) -> ModelPreferences {
        ModelPreferences {
            hints: if self.hints.is_empty() {
                None
            } else {
                Some(self.hints)
            },
            cost_priority: self.cost_priority,
            speed_priority: self.speed_priority,
            intelligence_priority: self.intelligence_priority,
        }
    }
}

impl Default for ModelPreferencesBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for convenient SamplingMessage creation
pub trait SamplingMessageExt {
    /// Create a system message with text
    fn system(content: impl Into<String>) -> SamplingMessage;
    /// Create a user message with text
    fn user_text(content: impl Into<String>) -> SamplingMessage;
    /// Create a user message with image
    fn user_image(data: impl Into<String>, mime_type: impl Into<String>) -> SamplingMessage;
    /// Create an assistant message with text
    fn assistant_text(content: impl Into<String>) -> SamplingMessage;
}

impl SamplingMessageExt for SamplingMessage {
    fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: ContentBlock::Text {
                text: content.into(),
            },
        }
    }

    fn user_text(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: ContentBlock::Text {
                text: content.into(),
            },
        }
    }

    fn user_image(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: ContentBlock::Image {
                data: data.into(),
                mime_type: mime_type.into(),
            },
        }
    }

    fn assistant_text(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: ContentBlock::Text {
                text: content.into(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_message_builder_basic() {
        let params = MessageBuilder::new()
            .max_tokens(2000)
            .system("You are a helpful assistant.")
            .user_text("Hello, how are you?")
            .assistant_text("I'm doing well, thank you!")
            .temperature(0.7)
            .build_params();

        assert_eq!(params.messages.len(), 3);
        assert_eq!(params.max_tokens, 2000);
        assert_eq!(params.temperature, Some(0.7));

        // Check first message (system)
        assert_eq!(params.messages[0].role, Role::System);
        if let ContentBlock::Text { text } = &params.messages[0].content {
            assert_eq!(text, "You are a helpful assistant.");
        } else {
            panic!("Expected text content");
        }
    }

    #[test]
    fn test_message_builder_model_preferences() {
        let params = MessageBuilder::new()
            .user_text("Test message")
            .with_model_preferences(|prefs| {
                prefs
                    .prefer_claude_sonnet()
                    .cost_priority(0.8)
                    .speed_priority(0.6)
                    .intelligence_priority(0.9)
            })
            .build_params();

        let preferences = params
            .model_preferences
            .expect("Expected model preferences");
        assert_eq!(preferences.hints.as_ref().unwrap().len(), 1);
        assert_eq!(
            preferences.hints.as_ref().unwrap()[0],
            ModelHint::Claude35Sonnet20241022
        );
        assert_eq!(preferences.cost_priority, Some(0.8));
        assert_eq!(preferences.speed_priority, Some(0.6));
        assert_eq!(preferences.intelligence_priority, Some(0.9));
    }

    #[test]
    fn test_message_builder_stop_sequences() {
        let params = MessageBuilder::new()
            .user_text("Generate some code")
            .stop_sequence("```")
            .stop_sequence("\n\n")
            .build_params();

        let sequences = params.stop_sequences.expect("Expected stop sequences");
        assert_eq!(sequences.len(), 2);
        assert_eq!(sequences[0], "```");
        assert_eq!(sequences[1], "\n\n");
    }

    #[test]
    fn test_message_builder_complete_request() {
        let request = MessageBuilder::new()
            .system_prompt("You are a coding assistant")
            .user_text("Write a function to calculate fibonacci numbers")
            .temperature(0.3)
            .max_tokens(500)
            .metadata(json!({"request_id": "12345"}))
            .build_request();

        assert_eq!(request.method, "sampling/createMessage");
        assert_eq!(request.params.max_tokens, 500);
        assert_eq!(request.params.temperature, Some(0.3));
        assert_eq!(
            request.params.system_prompt,
            Some("You are a coding assistant".to_string())
        );
        assert!(request.params.metadata.is_some());
    }

    #[test]
    fn test_model_preferences_builder() {
        let preferences = ModelPreferencesBuilder::new()
            .prefer_fast()
            .cost_priority(0.9)
            .speed_priority(0.8)
            .build();

        let hints = preferences.hints.expect("Expected hints");
        assert_eq!(hints.len(), 2);
        assert!(hints.contains(&ModelHint::Claude35Haiku20241022));
        assert!(hints.contains(&ModelHint::Gpt4oMini));
        assert_eq!(preferences.cost_priority, Some(0.9));
        assert_eq!(preferences.speed_priority, Some(0.8));
    }

    #[test]
    fn test_sampling_message_convenience_methods() {
        let system_msg = SamplingMessage::system("System prompt");
        assert_eq!(system_msg.role, Role::System);

        let user_msg = SamplingMessage::user_text("User input");
        assert_eq!(user_msg.role, Role::User);

        let assistant_msg = SamplingMessage::assistant_text("Assistant response");
        assert_eq!(assistant_msg.role, Role::Assistant);

        let image_msg = SamplingMessage::user_image("base64data", "image/png");
        assert_eq!(image_msg.role, Role::User);
        if let ContentBlock::Image { data, mime_type } = &image_msg.content {
            assert_eq!(data, "base64data");
            assert_eq!(mime_type, "image/png");
        } else {
            panic!("Expected image content");
        }
    }

    #[test]
    fn test_temperature_clamping() {
        let params = MessageBuilder::new()
            .user_text("Test")
            .temperature(5.0) // Should be clamped to 2.0
            .build_params();

        assert_eq!(params.temperature, Some(2.0));

        let params2 = MessageBuilder::new()
            .user_text("Test")
            .temperature(-1.0) // Should be clamped to 0.0
            .build_params();

        assert_eq!(params2.temperature, Some(0.0));
    }
}
