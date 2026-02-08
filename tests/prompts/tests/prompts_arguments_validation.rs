//! MCP Prompts Arguments Validation Tests
//!
//! Tests for MCP error codes and messages when required args are missing/invalid.
//! Verifies proper argument validation with structured MCP errors.

use serde_json::{json, Value};
use std::collections::HashMap;
use turul_mcp_protocol::prompts::*;
use turul_mcp_protocol::{McpError, McpResult};
use turul_mcp_server::handlers::{McpHandler, PromptsGetHandler};
use turul_mcp_server::McpPrompt;

// Enhanced test prompt with configurable required arguments
#[derive(Clone)]
struct ValidatedTestPrompt {
    pub name: String,
    pub description: String,
    pub required_arguments: Vec<PromptArgument>,
}

impl ValidatedTestPrompt {
    fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            required_arguments: vec![],
        }
    }

    fn with_required_argument(mut self, name: &str, description: &str) -> Self {
        self.required_arguments.push(PromptArgument {
            name: name.to_string(),
            title: None,
            description: Some(description.to_string()),
            required: Some(true), // Mark as required
        });
        self
    }

    fn with_optional_argument(mut self, name: &str, description: &str) -> Self {
        self.required_arguments.push(PromptArgument {
            name: name.to_string(),
            title: None,
            description: Some(description.to_string()),
            required: Some(false), // Mark as optional
        });
        self
    }
}

// Required trait implementations
use turul_mcp_builders::prelude::{
    HasIcons, HasPromptAnnotations, HasPromptArguments, HasPromptDescription, HasPromptMeta,
    HasPromptMetadata,
};

impl HasPromptMetadata for ValidatedTestPrompt {
    fn name(&self) -> &str {
        &self.name
    }
}

impl HasPromptDescription for ValidatedTestPrompt {
    fn description(&self) -> Option<&str> {
        Some(&self.description)
    }
}

impl HasPromptArguments for ValidatedTestPrompt {
    fn arguments(&self) -> Option<&Vec<PromptArgument>> {
        if self.required_arguments.is_empty() {
            None
        } else {
            Some(&self.required_arguments)
        }
    }
}

impl HasPromptAnnotations for ValidatedTestPrompt {}

impl HasPromptMeta for ValidatedTestPrompt {}

impl HasIcons for ValidatedTestPrompt {}

#[async_trait::async_trait]
impl McpPrompt for ValidatedTestPrompt {
    async fn render(&self, args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        let args = args.unwrap_or_default();

        // Create messages using provided arguments
        let mut messages = vec![PromptMessage::user_text(format!("Prompt: {}", self.name))];

        // Add argument values to assistant message
        let arg_summary = if args.is_empty() {
            "No arguments provided".to_string()
        } else {
            format!("Arguments: {:?}", args)
        };

        messages.push(PromptMessage::assistant_text(&arg_summary));
        Ok(messages)
    }
}

#[tokio::test]
async fn test_missing_required_single_argument() {
    let mut handler = PromptsGetHandler::new();

    let prompt = ValidatedTestPrompt::new(
        "single_required_arg_prompt",
        "A prompt with one required argument",
    )
    .with_required_argument("topic", "The topic to write about");

    handler = handler.add_prompt_arc(std::sync::Arc::new(prompt));

    // Test with missing required argument
    let get_params = json!({
        "name": "single_required_arg_prompt"
    });

    let result = handler.handle(Some(get_params)).await;
    assert!(result.is_err());

    let error = result.unwrap_err();
    match error {
        McpError::InvalidParameters(msg) => {
            assert!(msg.contains("Missing required argument 'topic'"));
            assert!(msg.contains("single_required_arg_prompt"));
        }
        _ => panic!("Expected InvalidParameters error, got: {:?}", error),
    }
}

#[tokio::test]
async fn test_missing_multiple_required_arguments() {
    let mut handler = PromptsGetHandler::new();

    let prompt = ValidatedTestPrompt::new(
        "multi_required_args_prompt",
        "A prompt with multiple required arguments",
    )
    .with_required_argument("topic", "The main topic")
    .with_required_argument("style", "Writing style")
    .with_required_argument("length", "Desired length");

    handler = handler.add_prompt_arc(std::sync::Arc::new(prompt));

    // Test with all required arguments missing
    let get_params = json!({
        "name": "multi_required_args_prompt"
    });

    let result = handler.handle(Some(get_params)).await;
    assert!(result.is_err());

    let error = result.unwrap_err();
    match error {
        McpError::InvalidParameters(msg) => {
            // Should report the first missing required argument
            assert!(msg.contains("Missing required argument"));
            assert!(msg.contains("multi_required_args_prompt"));
            // Should mention one of: topic, style, or length
            assert!(msg.contains("topic") || msg.contains("style") || msg.contains("length"));
        }
        _ => panic!("Expected InvalidParameters error, got: {:?}", error),
    }
}

#[tokio::test]
async fn test_partial_required_arguments() {
    let mut handler = PromptsGetHandler::new();

    let prompt = ValidatedTestPrompt::new(
        "partial_args_prompt",
        "A prompt testing partial argument provision",
    )
    .with_required_argument("required_arg1", "First required argument")
    .with_required_argument("required_arg2", "Second required argument")
    .with_optional_argument("optional_arg", "Optional argument");

    handler = handler.add_prompt_arc(std::sync::Arc::new(prompt));

    // Test with only one of two required arguments
    let get_params = json!({
        "name": "partial_args_prompt",
        "arguments": {
            "required_arg1": "value1",
            "optional_arg": "optional_value"
        }
    });

    let result = handler.handle(Some(get_params)).await;
    assert!(result.is_err());

    let error = result.unwrap_err();
    match error {
        McpError::InvalidParameters(msg) => {
            assert!(msg.contains("Missing required argument 'required_arg2'"));
        }
        _ => panic!(
            "Expected InvalidParameters error for missing required_arg2, got: {:?}",
            error
        ),
    }
}

#[tokio::test]
async fn test_all_required_arguments_provided() {
    let mut handler = PromptsGetHandler::new();

    let prompt = ValidatedTestPrompt::new(
        "complete_args_prompt",
        "A prompt with all required arguments provided",
    )
    .with_required_argument("topic", "The topic")
    .with_required_argument("style", "The style")
    .with_optional_argument("extras", "Extra information");

    handler = handler.add_prompt_arc(std::sync::Arc::new(prompt));

    // Test with all required arguments provided
    let get_params = json!({
        "name": "complete_args_prompt",
        "arguments": {
            "topic": "AI Safety",
            "style": "academic"
        }
    });

    let result = handler.handle(Some(get_params)).await;
    assert!(
        result.is_ok(),
        "Should succeed when all required arguments are provided"
    );

    let response: GetPromptResult = serde_json::from_value(result.unwrap()).unwrap();
    assert_eq!(response.messages.len(), 2);

    // Verify arguments were processed
    match &response.messages[1].content {
        ContentBlock::Text { text, .. } => {
            assert!(text.contains("AI Safety"));
            assert!(text.contains("academic"));
        }
        _ => panic!("Expected text content with argument values"),
    }
}

#[tokio::test]
async fn test_optional_arguments_handling() {
    let mut handler = PromptsGetHandler::new();

    let prompt =
        ValidatedTestPrompt::new("optional_args_prompt", "A prompt with optional arguments")
            .with_required_argument("required_field", "Must be provided")
            .with_optional_argument("optional_field1", "Can be omitted")
            .with_optional_argument("optional_field2", "Also optional");

    handler = handler.add_prompt_arc(std::sync::Arc::new(prompt));

    // Test with only required argument (optional ones omitted)
    let get_params = json!({
        "name": "optional_args_prompt",
        "arguments": {
            "required_field": "required_value"
        }
    });

    let result = handler.handle(Some(get_params)).await;
    assert!(
        result.is_ok(),
        "Should succeed when only required arguments are provided"
    );

    // Test with required + some optional arguments
    let get_params_with_optional = json!({
        "name": "optional_args_prompt",
        "arguments": {
            "required_field": "required_value",
            "optional_field1": "optional_value"
        }
    });

    let result = handler.handle(Some(get_params_with_optional)).await;
    assert!(
        result.is_ok(),
        "Should succeed with optional arguments included"
    );
}

#[tokio::test]
async fn test_prompt_without_arguments_schema() {
    let mut handler = PromptsGetHandler::new();

    // Prompt with no argument schema defined
    let prompt = ValidatedTestPrompt::new(
        "no_args_prompt",
        "A prompt that doesn't define any arguments",
    );

    handler = handler.add_prompt_arc(std::sync::Arc::new(prompt));

    // Should work with no arguments
    let get_params = json!({
        "name": "no_args_prompt"
    });

    let result = handler.handle(Some(get_params)).await;
    assert!(
        result.is_ok(),
        "Prompt without argument schema should work with no arguments"
    );

    // Should also work with arbitrary arguments (no validation)
    let get_params_with_args = json!({
        "name": "no_args_prompt",
        "arguments": {
            "arbitrary_arg": "arbitrary_value"
        }
    });

    let result = handler.handle(Some(get_params_with_args)).await;
    assert!(
        result.is_ok(),
        "Prompt without argument schema should accept any arguments"
    );
}

#[tokio::test]
async fn test_argument_validation_error_structure() {
    let mut handler = PromptsGetHandler::new();

    let prompt = ValidatedTestPrompt::new(
        "validation_error_prompt",
        "Prompt for testing error structure",
    )
    .with_required_argument("mandatory", "A mandatory field");

    handler = handler.add_prompt_arc(std::sync::Arc::new(prompt));

    let get_params = json!({
        "name": "validation_error_prompt"
    });

    let result = handler.handle(Some(get_params)).await;
    assert!(result.is_err());

    let error = result.unwrap_err();

    // Test that error can be converted to JSON-RPC error
    let json_rpc_error = error.to_error_object();

    // Should be InvalidParams error code (-32602)
    assert_eq!(json_rpc_error.code, -32602);

    // Verify the error message contains the expected content
    assert!(json_rpc_error.message.contains("Missing required argument"));
    assert!(json_rpc_error.message.contains("'mandatory'"));
    assert!(json_rpc_error.message.contains("validation_error_prompt"));
}

#[tokio::test]
async fn test_empty_string_arguments_handling() {
    let mut handler = PromptsGetHandler::new();

    let prompt = ValidatedTestPrompt::new(
        "empty_string_args_prompt",
        "Tests handling of empty string arguments",
    )
    .with_required_argument("content", "Required content field");

    handler = handler.add_prompt_arc(std::sync::Arc::new(prompt));

    // Test with empty string for required argument (should be valid - string exists)
    let get_params = json!({
        "name": "empty_string_args_prompt",
        "arguments": {
            "content": ""
        }
    });

    let result = handler.handle(Some(get_params)).await;
    assert!(
        result.is_ok(),
        "Empty string should satisfy required argument check"
    );
}

#[tokio::test]
async fn test_null_arguments_handling() {
    let mut handler = PromptsGetHandler::new();

    let prompt = ValidatedTestPrompt::new("null_args_prompt", "Tests null arguments handling")
        .with_required_argument("required_field", "A required field");

    handler = handler.add_prompt_arc(std::sync::Arc::new(prompt));

    // Test with null arguments object
    let get_params = json!({
        "name": "null_args_prompt",
        "arguments": null
    });

    let result = handler.handle(Some(get_params)).await;
    assert!(
        result.is_err(),
        "Null arguments should trigger missing required argument error"
    );
}
