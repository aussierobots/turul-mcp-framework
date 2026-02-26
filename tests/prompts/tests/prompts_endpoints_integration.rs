//! MCP Prompts Endpoints Integration Tests
//!
//! End-to-end integration tests for prompts/list and prompts/get endpoints
//! using direct handler calls to verify JSON-RPC compliance and MCP specification adherence.

use serde_json::{Value, json};
use std::collections::HashMap;
use turul_mcp_protocol::meta::*;
use turul_mcp_protocol::prompts::*;
use turul_mcp_server::handlers::{McpHandler, PromptsGetHandler, PromptsListHandler};
use turul_mcp_server::prelude::*;

// Test prompt implementations for integration testing
#[derive(Clone)]
struct SimpleTestPrompt {
    pub name: String,
    pub description: String,
    pub required_args: Vec<String>,
}

impl SimpleTestPrompt {
    fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            required_args: vec![],
        }
    }

    fn with_required_arg(mut self, arg_name: &str) -> Self {
        self.required_args.push(arg_name.to_string());
        self
    }
}

// Required trait implementations
use turul_mcp_builders::prelude::{
    HasPromptAnnotations, HasPromptArguments, HasPromptDescription, HasPromptMeta,
    HasPromptMetadata,
};
use turul_mcp_protocol::prompts::PromptArgument;
use turul_mcp_server::McpPrompt;

impl HasPromptMetadata for SimpleTestPrompt {
    fn name(&self) -> &str {
        &self.name
    }
}

impl HasPromptDescription for SimpleTestPrompt {
    fn description(&self) -> Option<&str> {
        Some(&self.description)
    }
}

impl HasPromptArguments for SimpleTestPrompt {
    fn arguments(&self) -> Option<&Vec<PromptArgument>> {
        // For testing, we'll create arguments on the fly
        // In real implementation, these would be stored
        None // Simplified for test
    }
}

impl HasPromptAnnotations for SimpleTestPrompt {}

impl HasPromptMeta for SimpleTestPrompt {}

impl HasIcons for SimpleTestPrompt {}

#[async_trait::async_trait]
impl McpPrompt for SimpleTestPrompt {
    async fn render(&self, args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        let args = args.unwrap_or_default();

        // Validate required arguments
        for required_arg in &self.required_args {
            if !args.contains_key(required_arg) {
                return Err(McpError::InvalidParameters(format!(
                    "Missing required argument: {}",
                    required_arg
                )));
            }
        }

        // Generate test messages
        let messages = vec![
            PromptMessage::user_text(format!("User message for prompt: {}", self.name)),
            PromptMessage::assistant_text(format!("Assistant response for prompt: {}", self.name)),
        ];

        Ok(messages)
    }
}

#[tokio::test]
async fn test_prompts_list_endpoint_integration() {
    let mut handler = PromptsListHandler::new();

    // Add test prompts
    for i in 1..=25 {
        let prompt = SimpleTestPrompt::new(
            &format!("test_prompt_{:02}", i),
            &format!("Test prompt number {} for integration testing", i),
        );
        handler = handler.add_prompt_arc(std::sync::Arc::new(prompt));
    }

    // Test prompts/list with no parameters (first page)
    let response = handler.handle(None).await.unwrap();
    let response_obj: PaginatedResponse<ListPromptsResult> =
        serde_json::from_value(response).unwrap();

    // Verify pagination structure
    assert_eq!(response_obj.data.prompts.len(), 25);
    assert!(response_obj.meta.is_some());

    let meta = response_obj.meta.as_ref().unwrap();
    assert_eq!(meta.total, Some(25));
    assert_eq!(meta.has_more, Some(false)); // All fit in one page
    assert!(meta.cursor.is_none()); // No more pages

    // Verify prompt structure
    let first_prompt = &response_obj.data.prompts[0];
    assert!(!first_prompt.name.is_empty());
    assert!(first_prompt.description.is_some());
    assert!(
        first_prompt
            .description
            .as_ref()
            .unwrap()
            .contains("integration testing")
    );
}

#[tokio::test]
async fn test_prompts_list_pagination_integration() {
    let mut handler = PromptsListHandler::new();

    // Add enough prompts to trigger pagination (more than 50)
    for i in 1..=75 {
        let prompt = SimpleTestPrompt::new(
            &format!("page_test_{:03}", i),
            &format!("Pagination test prompt {}", i),
        );
        handler = handler.add_prompt_arc(std::sync::Arc::new(prompt));
    }

    // Get first page
    let page1_response = handler.handle(None).await.unwrap();
    let page1_data: PaginatedResponse<ListPromptsResult> =
        serde_json::from_value(page1_response).unwrap();

    let page1_meta = page1_data.meta.as_ref().unwrap();
    assert_eq!(page1_data.data.prompts.len(), 50); // Default page size
    assert_eq!(page1_meta.has_more, Some(true));
    assert!(page1_meta.cursor.is_some());

    let cursor = page1_meta.cursor.as_ref().unwrap();

    // Get second page using cursor
    let page2_params = json!({ "cursor": cursor.as_str() });
    let page2_response = handler.handle(Some(page2_params)).await.unwrap();
    let page2_data: PaginatedResponse<ListPromptsResult> =
        serde_json::from_value(page2_response).unwrap();

    // Verify second page
    assert_eq!(page2_data.data.prompts.len(), 25); // Remaining items

    let page2_meta = page2_data.meta.as_ref().unwrap();
    assert_eq!(page2_meta.has_more, Some(false));
    assert!(page2_meta.cursor.is_none());

    // Verify no overlap between pages
    let page1_names: Vec<&String> = page1_data.data.prompts.iter().map(|p| &p.name).collect();
    let page2_names: Vec<&String> = page2_data.data.prompts.iter().map(|p| &p.name).collect();

    for page1_name in &page1_names {
        assert!(
            !page2_names.contains(page1_name),
            "Pages should not overlap"
        );
    }
}

#[tokio::test]
async fn test_prompts_get_endpoint_integration() {
    let mut handler = PromptsGetHandler::new();

    // Add a test prompt
    let prompt = SimpleTestPrompt::new(
        "integration_test_prompt",
        "A prompt for testing the get endpoint",
    );
    handler = handler.add_prompt_arc(std::sync::Arc::new(prompt));

    // Test prompts/get with valid prompt name
    let get_params = json!({
        "name": "integration_test_prompt"
    });

    let response = handler.handle(Some(get_params)).await.unwrap();
    let get_result: GetPromptResult = serde_json::from_value(response).unwrap();

    // Verify response structure
    assert!(get_result.description.is_some());
    assert_eq!(
        get_result.description.as_ref().unwrap(),
        "A prompt for testing the get endpoint"
    );
    assert_eq!(get_result.messages.len(), 2); // User + assistant messages

    // Verify message roles are MCP-compliant (only user/assistant, no system)
    assert_eq!(
        get_result.messages[0].role,
        turul_mcp_protocol::prompts::Role::User
    );
    assert_eq!(
        get_result.messages[1].role,
        turul_mcp_protocol::prompts::Role::Assistant
    );

    // Verify message content uses proper ContentBlock variants
    match &get_result.messages[0].content {
        ContentBlock::Text { text, .. } => {
            assert!(text.contains("integration_test_prompt"));
        }
        _ => panic!("Expected ContentBlock::Text variant"),
    }
}

#[tokio::test]
async fn test_prompts_get_with_meta_propagation() {
    let mut handler = PromptsGetHandler::new();

    // Add a test prompt
    let prompt =
        SimpleTestPrompt::new("meta_test_prompt", "A prompt for testing _meta propagation");
    handler = handler.add_prompt_arc(std::sync::Arc::new(prompt));

    // Test prompts/get with _meta in request
    let mut test_meta = HashMap::new();
    test_meta.insert("test_key".to_string(), json!("test_value"));
    test_meta.insert("request_id".to_string(), json!("req_12345"));

    let get_params = json!({
        "name": "meta_test_prompt",
        "_meta": test_meta
    });

    let response = handler.handle(Some(get_params)).await.unwrap();
    let get_result: GetPromptResult = serde_json::from_value(response).unwrap();

    // Verify _meta was propagated from request to response
    assert!(get_result.meta.is_some());
    let response_meta = get_result.meta.as_ref().unwrap();
    assert_eq!(response_meta.get("test_key").unwrap(), &json!("test_value"));
    assert_eq!(
        response_meta.get("request_id").unwrap(),
        &json!("req_12345")
    );
}

#[tokio::test]
async fn test_prompts_get_missing_required_arguments() {
    let mut handler = PromptsGetHandler::new();

    // Add a prompt with required arguments (we'll simulate this in the prompt implementation)
    let prompt = SimpleTestPrompt::new("required_args_prompt", "A prompt that requires arguments")
        .with_required_arg("topic")
        .with_required_arg("style");

    handler = handler.add_prompt_arc(std::sync::Arc::new(prompt));

    // Test prompts/get without required arguments - should return validation error
    let get_params = json!({
        "name": "required_args_prompt"
    });

    let result = handler.handle(Some(get_params)).await;
    assert!(
        result.is_err(),
        "Should return error for missing required arguments"
    );

    // Verify it's an InvalidParameters error
    let error = result.unwrap_err();
    match error {
        McpError::InvalidParameters(msg) => {
            assert!(msg.contains("Missing required argument"));
        }
        _ => panic!("Expected InvalidParameters error, got: {:?}", error),
    }
}

#[tokio::test]
async fn test_prompts_get_nonexistent_prompt() {
    let handler = PromptsGetHandler::new(); // Empty handler

    // Test prompts/get with nonexistent prompt
    let get_params = json!({
        "name": "nonexistent_prompt"
    });

    let result = handler.handle(Some(get_params)).await;
    assert!(
        result.is_err(),
        "Should return error for nonexistent prompt"
    );

    // Verify it's the correct error type
    let error = result.unwrap_err();
    match error {
        McpError::InvalidParameterType {
            param,
            expected,
            actual,
        } => {
            assert_eq!(param, "name");
            assert_eq!(expected, "existing prompt name");
            assert_eq!(actual, "nonexistent_prompt");
        }
        _ => panic!("Expected InvalidParameterType error, got: {:?}", error),
    }
}

#[tokio::test]
async fn test_prompts_get_missing_parameters() {
    let handler = PromptsGetHandler::new();

    // Test prompts/get with no parameters at all
    let result = handler.handle(None).await;
    assert!(
        result.is_err(),
        "Should return error for missing parameters"
    );

    // Verify it's a MissingParameter error
    let error = result.unwrap_err();
    match error {
        McpError::MissingParameter(param) => {
            assert_eq!(param, "GetPromptParams");
        }
        _ => panic!("Expected MissingParameter error, got: {:?}", error),
    }
}

#[tokio::test]
async fn test_stable_prompt_ordering() {
    let mut handler = PromptsListHandler::new();

    // Add prompts in non-alphabetical order
    let prompt_names = vec!["zebra_prompt", "alpha_prompt", "beta_prompt"];
    for name in prompt_names {
        let prompt = SimpleTestPrompt::new(name, &format!("Description for {}", name));
        handler = handler.add_prompt_arc(std::sync::Arc::new(prompt));
    }

    // Get list multiple times
    let response1 = handler.handle(None).await.unwrap();
    let response2 = handler.handle(None).await.unwrap();

    let data1: PaginatedResponse<ListPromptsResult> = serde_json::from_value(response1).unwrap();
    let data2: PaginatedResponse<ListPromptsResult> = serde_json::from_value(response2).unwrap();

    // Verify consistent ordering
    assert_eq!(data1.data.prompts.len(), data2.data.prompts.len());
    for (p1, p2) in data1.data.prompts.iter().zip(data2.data.prompts.iter()) {
        assert_eq!(p1.name, p2.name, "Prompt ordering must be stable");
    }

    // Verify prompts are sorted by name (stable pagination requirement)
    let names: Vec<&String> = data1.data.prompts.iter().map(|p| &p.name).collect();
    let mut sorted_names = names.clone();
    sorted_names.sort();
    assert_eq!(names, sorted_names, "Prompts should be sorted by name");
}
