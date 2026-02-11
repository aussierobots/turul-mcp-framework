//! MCP Prompt Trait
//!
//! This module defines the high-level trait for implementing MCP prompts.

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use turul_mcp_builders::prelude::*;
use turul_mcp_protocol::{
    McpResult,
    prompts::{GetPromptResult, PromptMessage},
};

/// High-level trait for implementing MCP prompts
///
/// McpPrompt extends PromptDefinition with execution capabilities.
/// All metadata is provided by the PromptDefinition trait, ensuring
/// consistency between concrete Prompt structs and dynamic implementations.
#[async_trait]
pub trait McpPrompt: PromptDefinition + Send + Sync {
    /// Render the prompt with the given arguments
    ///
    /// This method processes the arguments and returns the rendered prompt messages.
    /// The implementation should substitute arguments into templates and generate
    /// the final prompt content.
    ///
    /// Default implementation returns a simple template message. Override for custom logic.
    async fn render(&self, _args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        // Default implementation - simple template message
        let message = format!(
            "Prompt: {} - {}",
            self.name(),
            self.description().unwrap_or("Generated prompt")
        );
        Ok(vec![PromptMessage::text(message)])
    }

    /// Optional: Check if this prompt handler can handle the given arguments
    ///
    /// This allows for conditional prompt handling based on argument content,
    /// complexity, or other factors.
    fn can_handle(&self, args: &HashMap<String, Value>) -> bool {
        // Default implementation validates required arguments
        if let Some(prompt_args) = self.arguments() {
            for arg in prompt_args {
                if arg.required.unwrap_or(false) && !args.contains_key(&arg.name) {
                    return false;
                }
            }
        }
        true
    }

    /// Optional: Get prompt priority for request routing
    ///
    /// Higher priority handlers are tried first when multiple handlers
    /// can handle the same prompt.
    fn priority(&self) -> u32 {
        0
    }

    /// Optional: Validate arguments before rendering
    ///
    /// This method performs argument validation beyond basic required/optional checks.
    async fn validate_args(&self, _args: &HashMap<String, Value>) -> McpResult<()> {
        // Default implementation: no validation
        Ok(())
    }

    /// Optional: Transform rendered messages before returning
    ///
    /// This allows for post-processing of rendered messages, such as formatting,
    /// optimization, or additional content enhancement.
    async fn transform_messages(
        &self,
        messages: Vec<PromptMessage>,
    ) -> McpResult<Vec<PromptMessage>> {
        Ok(messages)
    }

    /// Convenience method to render and create a complete response
    async fn get_response(
        &self,
        args: Option<HashMap<String, Value>>,
    ) -> McpResult<GetPromptResult> {
        // Validate arguments if provided
        if let Some(ref args) = args {
            self.validate_args(args).await?;
        }

        // Render the messages
        let messages = self.render(args).await?;

        // Transform if needed
        let final_messages = self.transform_messages(messages).await?;

        // Create response
        let mut response = GetPromptResult::new(final_messages);

        // Add description if available
        if let Some(description) = self.description() {
            response = response.with_description(description);
        }

        Ok(response)
    }
}

/// Convert an McpPrompt trait object to a Prompt descriptor
///
/// This is a convenience function for converting prompt definitions
/// to protocol descriptors.
pub fn prompt_to_descriptor(prompt: &dyn McpPrompt) -> turul_mcp_protocol::prompts::Prompt {
    prompt.to_prompt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use std::collections::HashMap;
    use turul_mcp_protocol::prompts::{PromptAnnotations, PromptArgument};
    // HasPromptMetadata, HasPromptDescription, etc.

    struct TestPrompt {
        name: String,
        description: String,
        arguments: Vec<PromptArgument>,
        template: String,
    }

    // Implement fine-grained traits
    impl HasPromptMetadata for TestPrompt {
        fn name(&self) -> &str {
            &self.name
        }
    }

    impl HasPromptDescription for TestPrompt {
        fn description(&self) -> Option<&str> {
            Some(&self.description)
        }
    }

    impl HasPromptArguments for TestPrompt {
        fn arguments(&self) -> Option<&Vec<PromptArgument>> {
            Some(&self.arguments)
        }
    }

    impl HasPromptAnnotations for TestPrompt {
        fn annotations(&self) -> Option<&PromptAnnotations> {
            None
        }
    }

    impl HasPromptMeta for TestPrompt {
        fn prompt_meta(&self) -> Option<&std::collections::HashMap<String, Value>> {
            None
        }
    }

    impl HasIcons for TestPrompt {}

    // PromptDefinition automatically implemented via blanket impl!

    impl TestPrompt {
        fn render_messages(
            &self,
            args: Option<&HashMap<String, Value>>,
        ) -> Result<Vec<PromptMessage>, String> {
            let mut template = self.template.clone();

            if let Some(args) = args {
                for (key, value) in args {
                    let placeholder = format!("{{{}}}", key);
                    let value_str = match value {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        _ => value.to_string(),
                    };
                    template = template.replace(&placeholder, &value_str);
                }
            }

            Ok(vec![PromptMessage::user_text(template)])
        }
    }

    #[async_trait]
    impl McpPrompt for TestPrompt {
        async fn render(
            &self,
            args: Option<HashMap<String, Value>>,
        ) -> McpResult<Vec<PromptMessage>> {
            match self.render_messages(args.as_ref()) {
                Ok(messages) => Ok(messages),
                Err(msg) => Err(turul_mcp_protocol::McpError::validation(&msg)),
            }
        }
    }

    #[test]
    fn test_prompt_trait() {
        let prompt = TestPrompt {
            name: "essay_prompt".to_string(),
            description: "Generate an essay prompt".to_string(),
            arguments: vec![
                PromptArgument::new("topic")
                    .with_description("The essay topic")
                    .required(),
            ],
            template: "Write an essay about {topic}.".to_string(),
        };

        assert_eq!(prompt.name(), "essay_prompt");
        assert_eq!(prompt.description(), Some("Generate an essay prompt"));
        assert!(prompt.arguments().is_some());
    }

    #[test]
    fn test_prompt_to_descriptor() {
        let prompt = TestPrompt {
            name: "test_prompt".to_string(),
            description: "A test prompt".to_string(),
            arguments: vec![],
            template: "Test template".to_string(),
        };

        let descriptor = prompt_to_descriptor(&prompt);

        assert_eq!(descriptor.name, "test_prompt");
        assert_eq!(descriptor.description, Some("A test prompt".to_string()));
    }

    #[tokio::test]
    async fn test_prompt_rendering() {
        let prompt = TestPrompt {
            name: "essay_prompt".to_string(),
            description: "Essay writing prompt".to_string(),
            arguments: vec![
                PromptArgument::new("topic")
                    .with_description("The essay topic")
                    .required(),
            ],
            template: "Write an essay about {topic}.".to_string(),
        };

        let mut args = HashMap::new();
        args.insert("topic".to_string(), json!("artificial intelligence"));

        let messages = prompt.render(Some(args)).await.unwrap();
        assert_eq!(messages.len(), 1);

        // Check the content of the message
        let turul_mcp_protocol::prompts::ContentBlock::Text { text, .. } = &messages[0].content
        else {
            panic!("Expected text message, got: {:?}", messages[0].content);
        };
        assert!(text.contains("artificial intelligence"));
    }

    #[tokio::test]
    async fn test_argument_validation() {
        let prompt = TestPrompt {
            name: "essay_prompt".to_string(),
            description: "Essay writing prompt".to_string(),
            arguments: vec![
                PromptArgument::new("topic")
                    .with_description("The essay topic")
                    .required(),
            ],
            template: "Write an essay about {topic}.".to_string(),
        };

        // Valid arguments
        let valid_args = HashMap::from([("topic".to_string(), json!("AI"))]);
        assert!(prompt.can_handle(&valid_args));

        // Missing required argument
        let invalid_args = HashMap::new();
        assert!(!prompt.can_handle(&invalid_args));
    }

    #[tokio::test]
    async fn test_get_response() {
        let prompt = TestPrompt {
            name: "greeting".to_string(),
            description: "A greeting prompt".to_string(),
            arguments: vec![],
            template: "Hello, world!".to_string(),
        };

        let response = prompt.get_response(None).await.unwrap();

        assert_eq!(response.messages.len(), 1);
        assert_eq!(response.description, Some("A greeting prompt".to_string()));

        // Check the content of the message
        let turul_mcp_protocol::prompts::ContentBlock::Text { text, .. } =
            &response.messages[0].content
        else {
            panic!(
                "Expected text message, got: {:?}",
                response.messages[0].content
            );
        };
        assert_eq!(text, "Hello, world!");
    }
}
