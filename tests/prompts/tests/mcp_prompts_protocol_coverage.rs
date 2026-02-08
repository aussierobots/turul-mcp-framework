//! Comprehensive MCP Prompts Protocol Coverage Tests
//!
//! This test file covers ALL structs and enums from turul-mcp-protocol-2025-06-18/src/prompts.rs
//! ensuring complete MCP 2025-06-18 specification compliance.

use serde_json::json;
use std::collections::HashMap;
use turul_mcp_protocol::meta::Cursor;
use turul_mcp_protocol::prompts::*;
use turul_mcp_protocol::{Annotations, BlobResourceContents, TextResourceContents};
use turul_mcp_builders::prelude::*;  // PromptDefinition and other framework traits

#[tokio::test]
async fn test_content_block_text_variant() {
    // Test ContentBlock::Text variant
    let text_block = ContentBlock::Text {
        text: "Hello, this is a text content block.".to_string(),
        annotations: None,
        meta: None,
    };

    match text_block {
        ContentBlock::Text { text, .. } => {
            assert_eq!(text, "Hello, this is a text content block.");
        }
        _ => panic!("Expected ContentBlock::Text variant"),
    }
}

#[tokio::test]
async fn test_content_block_image_variant() {
    // Test ContentBlock::Image variant
    let base64_image = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

    let image_block = ContentBlock::Image {
        data: base64_image.to_string(),
        mime_type: "image/png".to_string(),
        annotations: None,
        meta: None,
    };

    match image_block {
        ContentBlock::Image {
            data, mime_type, ..
        } => {
            assert_eq!(data, base64_image);
            assert_eq!(mime_type, "image/png");
        }
        _ => panic!("Expected ContentBlock::Image variant"),
    }
}

#[tokio::test]
async fn test_content_block_resource_link_variant() {
    // Test ContentBlock::ResourceLink variant
    let resource_ref = ResourceReference {
        uri: "file:///document.pdf".to_string(),
        name: "important_document".to_string(),
        title: Some("Important Document".to_string()),
        description: Some("A critical document for the project".to_string()),
        mime_type: Some("application/pdf".to_string()),
        annotations: None,
        meta: None,
    };

    let resource_link_block = ContentBlock::ResourceLink {
        resource: resource_ref.clone(),
        annotations: None,
        meta: None,
    };

    match resource_link_block {
        ContentBlock::ResourceLink { resource, .. } => {
            assert_eq!(resource.uri, "file:///document.pdf");
            assert_eq!(resource.name, "important_document");
            assert_eq!(resource.title, Some("Important Document".to_string()));
            assert_eq!(resource.mime_type, Some("application/pdf".to_string()));
        }
        _ => panic!("Expected ContentBlock::ResourceLink variant"),
    }
}

#[tokio::test]
async fn test_content_block_resource_embedded_variant() {
    // Test ContentBlock::Resource variant (embedded resource)
    let embedded_resource = ResourceContents::Text(TextResourceContents {
        uri: "file:///config.json".to_string(),
        mime_type: Some("application/json".to_string()),
        text: r#"{"version": "1.0", "debug": false}"#.to_string(),
        meta: None,
    });

    let mut meta = HashMap::new();
    meta.insert("source".to_string(), json!("configuration"));

    let resource_block = ContentBlock::Resource {
        resource: embedded_resource,
        annotations: Some(Annotations {
            audience: Some(vec!["user".to_string()]),
            priority: None,
            last_modified: None,
        }),
        meta: Some(meta),
    };

    match resource_block {
        ContentBlock::Resource {
            resource,
            annotations,
            meta,
        } => {
            match resource {
                ResourceContents::Text(text_contents) => {
                    assert_eq!(text_contents.uri, "file:///config.json");
                    assert!(text_contents.text.contains("version"));
                }
                ResourceContents::Blob(_) => panic!("Expected Text resource"),
            }
            assert!(annotations.is_some());
            assert!(meta.is_some());
        }
        _ => panic!("Expected ContentBlock::Resource variant"),
    }
}

#[tokio::test]
async fn test_resource_contents_text_variant() {
    // Test ResourceContents::Text variant
    let text_resource = ResourceContents::Text(TextResourceContents {
        uri: "file:///readme.md".to_string(),
        mime_type: Some("text/markdown".to_string()),
        text: "# Project README\n\nThis is the project documentation.".to_string(),
        meta: None,
    });

    match text_resource {
        ResourceContents::Text(text_contents) => {
            assert_eq!(text_contents.uri, "file:///readme.md");
            assert_eq!(text_contents.mime_type, Some("text/markdown".to_string()));
            assert!(text_contents.text.contains("Project README"));
        }
        ResourceContents::Blob(_) => panic!("Expected Text resource"),
    }
}

#[tokio::test]
async fn test_resource_contents_blob_variant() {
    // Test ResourceContents::Blob variant
    let base64_data = "UEsDBBQAAAAIAO2N8VICAAAA";

    let blob_resource = ResourceContents::Blob(BlobResourceContents {
        uri: "file:///archive.zip".to_string(),
        mime_type: Some("application/zip".to_string()),
        blob: base64_data.to_string(),
        meta: None,
    });

    match blob_resource {
        ResourceContents::Blob(blob_contents) => {
            assert_eq!(blob_contents.uri, "file:///archive.zip");
            assert_eq!(blob_contents.mime_type, Some("application/zip".to_string()));
            assert_eq!(blob_contents.blob, base64_data);
        }
        ResourceContents::Text(_) => panic!("Expected Blob resource"),
    }
}

#[tokio::test]
async fn test_role_enum_user_and_assistant_only() {
    // Test Role enum - MCP spec allows only User and Assistant
    let user_role = Role::User;
    let assistant_role = Role::Assistant;

    // Test serialization
    let user_json = serde_json::to_string(&user_role).unwrap();
    let assistant_json = serde_json::to_string(&assistant_role).unwrap();

    assert_eq!(user_json, "\"user\"");
    assert_eq!(assistant_json, "\"assistant\"");

    // Test deserialization
    let user_parsed: Role = serde_json::from_str("\"user\"").unwrap();
    let assistant_parsed: Role = serde_json::from_str("\"assistant\"").unwrap();

    assert_eq!(user_parsed, Role::User);
    assert_eq!(assistant_parsed, Role::Assistant);
}

#[tokio::test]
async fn test_prompt_message_construction() {
    // Test PromptMessage construction with different methods
    let user_text = PromptMessage::user_text("What is the weather like?");
    let assistant_text = PromptMessage::assistant_text("I'll help you check the weather.");
    let user_image = PromptMessage::user_image("base64imagedata", "image/jpeg");

    // Test user text message
    assert_eq!(user_text.role, Role::User);
    match user_text.content {
        ContentBlock::Text { text, .. } => assert_eq!(text, "What is the weather like?"),
        _ => panic!("Expected Text content"),
    }

    // Test assistant text message
    assert_eq!(assistant_text.role, Role::Assistant);
    match assistant_text.content {
        ContentBlock::Text { text, .. } => assert!(text.contains("help you")),
        _ => panic!("Expected Text content"),
    }

    // Test user image message
    assert_eq!(user_image.role, Role::User);
    match user_image.content {
        ContentBlock::Image {
            data, mime_type, ..
        } => {
            assert_eq!(data, "base64imagedata");
            assert_eq!(mime_type, "image/jpeg");
        }
        _ => panic!("Expected Image content"),
    }

    // Test backward compatibility with text() method
    let compat_text = PromptMessage::text("Backward compatible text");
    assert_eq!(compat_text.role, Role::User); // Defaults to user
}

#[tokio::test]
async fn test_prompt_argument_validation() {
    // Test PromptArgument with various configurations
    let required_arg = PromptArgument::new("topic")
        .with_title("Essay Topic")
        .with_description("The main topic for the essay")
        .required();

    let optional_arg = PromptArgument::new("style")
        .with_description("Writing style preference")
        .optional();

    let minimal_arg = PromptArgument::new("minimal");

    // Test required argument
    assert_eq!(required_arg.name, "topic");
    assert_eq!(required_arg.title, Some("Essay Topic".to_string()));
    assert_eq!(
        required_arg.description,
        Some("The main topic for the essay".to_string())
    );
    assert_eq!(required_arg.required, Some(true));

    // Test optional argument
    assert_eq!(optional_arg.name, "style");
    assert_eq!(optional_arg.required, Some(false));

    // Test minimal argument
    assert_eq!(minimal_arg.name, "minimal");
    assert!(minimal_arg.title.is_none());
    assert!(minimal_arg.required.is_none());
}

#[tokio::test]
async fn test_prompt_struct_complete_fields() {
    // Test Prompt struct with all fields populated
    let arguments = vec![
        PromptArgument::new("input")
            .with_description("Input text")
            .required(),
        PromptArgument::new("format")
            .with_description("Output format")
            .optional(),
    ];

    let mut meta = HashMap::new();
    meta.insert("category".to_string(), json!("text_processing"));
    meta.insert("version".to_string(), json!("2.0"));

    let prompt = Prompt::new("text_processor")
        .with_title("Text Processing Prompt")
        .with_description("Processes text according to specified parameters")
        .with_arguments(arguments)
        .with_meta(meta);

    assert_eq!(prompt.name, "text_processor");
    assert_eq!(prompt.title, Some("Text Processing Prompt".to_string()));
    assert_eq!(
        prompt.description,
        Some("Processes text according to specified parameters".to_string())
    );
    assert!(prompt.arguments.is_some());
    assert_eq!(prompt.arguments.as_ref().unwrap().len(), 2);
    assert!(prompt.meta.is_some());
}

#[tokio::test]
async fn test_list_prompts_request_params() {
    // Test ListPromptsRequest and ListPromptsParams
    let cursor = Cursor::from("prompts_page_2");
    let mut meta = HashMap::new();
    meta.insert("filter_category".to_string(), json!("text"));

    let request = ListPromptsRequest::new()
        .with_cursor(cursor.clone())
        .with_meta(meta);

    assert_eq!(request.method, "prompts/list");
    assert!(request.params.cursor.is_some());
    assert_eq!(
        request.params.cursor.as_ref().unwrap().as_str(),
        "prompts_page_2"
    );
    assert!(request.params.meta.is_some());
}

#[tokio::test]
async fn test_list_prompts_result_pagination() {
    // Test ListPromptsResult with pagination
    let prompts = vec![
        Prompt::new("prompt1").with_description("First prompt"),
        Prompt::new("prompt2").with_description("Second prompt"),
        Prompt::new("prompt3").with_description("Third prompt"),
    ];

    let next_cursor = Cursor::from("next_prompts_page");
    let mut meta = HashMap::new();
    meta.insert("total_available".to_string(), json!("15")); // MCP spec requires string values

    let result = ListPromptsResult::new(prompts)
        .with_next_cursor(next_cursor)
        .with_meta(meta);

    assert_eq!(result.prompts.len(), 3);
    assert!(result.next_cursor.is_some());
    assert_eq!(
        result.next_cursor.as_ref().unwrap().as_str(),
        "next_prompts_page"
    );
    assert!(result.meta.is_some());
}

#[tokio::test]
async fn test_get_prompt_request_with_arguments() {
    // Test GetPromptRequest with string arguments (per MCP spec)
    let mut arguments = HashMap::new();
    arguments.insert("topic".to_string(), "Artificial Intelligence".to_string());
    arguments.insert("length".to_string(), "500 words".to_string());
    arguments.insert("style".to_string(), "academic".to_string());

    let mut meta = HashMap::new();
    meta.insert("user_id".to_string(), json!("user_123"));

    let request = GetPromptRequest::new("essay_prompt")
        .with_arguments(arguments.clone())
        .with_meta(meta);

    assert_eq!(request.method, "prompts/get");
    assert_eq!(request.params.name, "essay_prompt");
    assert!(request.params.arguments.is_some());
    assert!(request.params.meta.is_some());

    // Verify string-to-string mapping per MCP spec
    let req_args = request.params.arguments.as_ref().unwrap();
    assert_eq!(
        req_args.get("topic"),
        Some(&"Artificial Intelligence".to_string())
    );
    assert_eq!(req_args.get("length"), Some(&"500 words".to_string()));
}

#[tokio::test]
async fn test_get_prompt_result_with_conversation() {
    // Test GetPromptResult with user-assistant conversation
    let messages = vec![
        PromptMessage::user_text("Please write an essay about {topic} in {style} style."),
        PromptMessage::assistant_text(
            "I'll help you write an essay. Let me structure it properly.",
        ),
        PromptMessage::user_text("Make sure to include key points and examples."),
        PromptMessage::assistant_text(
            "Certainly! I'll include relevant examples and key arguments.",
        ),
    ];

    let mut meta = HashMap::new();
    meta.insert("template_version".to_string(), json!("2.1"));
    meta.insert("generated_at".to_string(), json!("2024-01-01T12:00:00Z"));

    let result = GetPromptResult::new(messages)
        .with_description("Essay writing prompt with interactive guidance")
        .with_meta(meta);

    assert_eq!(result.messages.len(), 4);
    assert!(result.description.is_some());
    assert!(result.meta.is_some());

    // Verify role alternation
    assert_eq!(result.messages[0].role, Role::User);
    assert_eq!(result.messages[1].role, Role::Assistant);
    assert_eq!(result.messages[2].role, Role::User);
    assert_eq!(result.messages[3].role, Role::Assistant);
}

#[tokio::test]
async fn test_prompt_template_variable_substitution() {
    // Test variable substitution patterns in prompts
    let template_message = PromptMessage::user_text(
        "Analyze the following {data_type} data: {data_content}. Focus on {analysis_aspect}.",
    );

    match template_message.content {
        ContentBlock::Text { text, .. } => {
            assert!(text.contains("{data_type}"));
            assert!(text.contains("{data_content}"));
            assert!(text.contains("{analysis_aspect}"));
        }
        _ => panic!("Expected Text content with template variables"),
    }

    // Test arguments for variable substitution
    let mut template_args = HashMap::new();
    template_args.insert("data_type".to_string(), "financial".to_string());
    template_args.insert("data_content".to_string(), "Q3 revenue report".to_string());
    template_args.insert("analysis_aspect".to_string(), "growth trends".to_string());

    let params = GetPromptParams::new("analysis_template").with_arguments(template_args);

    assert!(params.arguments.is_some());
    let args = params.arguments.as_ref().unwrap();
    assert_eq!(args.get("data_type"), Some(&"financial".to_string()));
}

#[tokio::test]
async fn test_resource_reference_complete_fields() {
    // Test ResourceReference with all fields
    let resource_ref = ResourceReference {
        uri: "file:///research/paper.pdf".to_string(),
        name: "research_paper".to_string(),
        title: Some("AI Research Paper".to_string()),
        description: Some("Comprehensive research on neural networks".to_string()),
        mime_type: Some("application/pdf".to_string()),
        annotations: None,
        meta: None,
    };

    assert_eq!(resource_ref.uri, "file:///research/paper.pdf");
    assert_eq!(resource_ref.name, "research_paper");
    assert_eq!(resource_ref.title, Some("AI Research Paper".to_string()));
    assert!(resource_ref.description.is_some());
    assert_eq!(resource_ref.mime_type, Some("application/pdf".to_string()));
}

#[tokio::test]
async fn test_prompt_annotations() {
    // Test PromptAnnotations
    let annotations = PromptAnnotations::new().with_title("Custom Prompt Title");

    assert_eq!(annotations.title, Some("Custom Prompt Title".to_string()));
}

#[tokio::test]
async fn test_prompt_trait_implementations() {
    // Test that Prompt struct implements all required traits
    let prompt = Prompt::new("test_prompt")
        .with_title("Test Prompt")
        .with_description("A prompt for testing trait implementations");

    // Test PromptDefinition trait methods
    assert_eq!(prompt.name(), "test_prompt");
    assert_eq!(prompt.title(), Some("Test Prompt"));
    assert_eq!(
        prompt.description(),
        Some("A prompt for testing trait implementations")
    );
    assert!(prompt.arguments().is_none());
    assert!(prompt.annotations().is_none());
    assert!(prompt.prompt_meta().is_none());

    // Test display_name method (title precedence)
    assert_eq!(prompt.display_name(), "Test Prompt");

    // Test prompt without title
    let no_title_prompt = Prompt::new("no_title");
    assert_eq!(no_title_prompt.display_name(), "no_title");

    // Test conversion to Prompt struct
    let converted = prompt.to_prompt();
    assert_eq!(converted.name, "test_prompt");
    assert_eq!(converted.title, Some("Test Prompt".to_string()));
}

#[tokio::test]
async fn test_serialization_round_trip_all_structs() {
    // Test serialization/deserialization for all major structs

    // Prompt
    let prompt = Prompt::new("serialize_test").with_description("Test prompt serialization");
    let prompt_json = serde_json::to_string(&prompt).unwrap();
    let prompt_parsed: Prompt = serde_json::from_str(&prompt_json).unwrap();
    assert_eq!(prompt.name, prompt_parsed.name);

    // PromptArgument
    let arg = PromptArgument::new("test_arg").required();
    let arg_json = serde_json::to_string(&arg).unwrap();
    let arg_parsed: PromptArgument = serde_json::from_str(&arg_json).unwrap();
    assert_eq!(arg.name, arg_parsed.name);

    // PromptMessage
    let message = PromptMessage::user_text("Test message");
    let message_json = serde_json::to_string(&message).unwrap();
    let message_parsed: PromptMessage = serde_json::from_str(&message_json).unwrap();
    assert_eq!(message.role, message_parsed.role);

    // ContentBlock::Text
    let text_block = ContentBlock::Text {
        text: "Test content".to_string(),
        annotations: None,
        meta: None,
    };
    let text_json = serde_json::to_string(&text_block).unwrap();
    let text_parsed: ContentBlock = serde_json::from_str(&text_json).unwrap();
    match text_parsed {
        ContentBlock::Text { text, .. } => assert_eq!(text, "Test content"),
        _ => panic!("Expected Text block"),
    }

    // Role enum
    let role = Role::Assistant;
    let role_json = serde_json::to_string(&role).unwrap();
    let role_parsed: Role = serde_json::from_str(&role_json).unwrap();
    assert_eq!(role, role_parsed);
}

#[tokio::test]
async fn test_mcp_error_handling_prompt_not_found() {
    // Test proper error handling with MCP-compliant error codes
    use turul_mcp_json_rpc_server::error::JsonRpcError;

    let error = JsonRpcError::invalid_params(1.into(), "nonexistent_prompt");
    assert_eq!(error.error.code, -32602); // JSON-RPC InvalidParams code
    assert!(error.error.message.contains("nonexistent_prompt"));
    // data field is optional for error details
    // assert!(error.error.data.is_some()); // Not guaranteed
}

#[tokio::test]
async fn test_edge_cases_and_robustness() {
    // Test edge cases for robustness

    // Empty prompt name
    let empty_name_prompt = Prompt::new("");
    assert_eq!(empty_name_prompt.name, "");

    // Very long prompt names
    let long_name = "a".repeat(1000);
    let long_name_prompt = Prompt::new(&long_name);
    assert_eq!(long_name_prompt.name.len(), 1000);

    // Empty text content
    let empty_text = PromptMessage::user_text("");
    match empty_text.content {
        ContentBlock::Text { text, .. } => assert_eq!(text, ""),
        _ => panic!("Expected empty text"),
    }

    // Special characters in content
    let special_chars =
        PromptMessage::assistant_text("Special chars: áéíóú 中文 🚀 \"quotes\" 'apostrophes'");
    match special_chars.content {
        ContentBlock::Text { text, .. } => assert!(text.contains("áéíóú")),
        _ => panic!("Expected special character text"),
    }

    // Minimal base64 image
    let minimal_image = PromptMessage::user_image("AA==", "image/gif");
    match minimal_image.content {
        ContentBlock::Image {
            data, mime_type, ..
        } => {
            assert_eq!(data, "AA==");
            assert_eq!(mime_type, "image/gif");
        }
        _ => panic!("Expected minimal image"),
    }

    // Empty argument description
    let empty_desc_arg = PromptArgument::new("test").with_description("");
    assert_eq!(empty_desc_arg.description, Some("".to_string()));
}

#[tokio::test]
async fn test_complex_content_block_combinations() {
    // Test complex combinations of ContentBlock variants in messages
    let complex_messages = [
        PromptMessage::user_text("Please analyze this document:"),
        PromptMessage {
            role: Role::User,
            content: ContentBlock::ResourceLink {
                resource: ResourceReference {
                    uri: "file:///analysis/data.csv".to_string(),
                    name: "dataset".to_string(),
                    title: Some("Analysis Dataset".to_string()),
                    description: Some("Primary dataset for analysis".to_string()),
                    mime_type: Some("text/csv".to_string()),
                    annotations: None,
                    meta: None,
                },
                annotations: None,
                meta: None,
            },
        },
        PromptMessage::assistant_text("I'll analyze the dataset. Here's the summary:"),
        PromptMessage {
            role: Role::Assistant,
            content: ContentBlock::Resource {
                resource: ResourceContents::Text(TextResourceContents {
                    uri: "memory://analysis_result".to_string(),
                    mime_type: Some("application/json".to_string()),
                    text: r#"{"rows": 1000, "columns": 5, "summary": "comprehensive"}"#.to_string(),
                    meta: None,
                }),
                annotations: Some(Annotations {
                    audience: Some(vec!["user".to_string()]),
                    priority: None,
                    last_modified: None,
                }),
                meta: None,
            },
        },
        PromptMessage {
            role: Role::User,
            content: ContentBlock::Image {
                data: "base64chartdata".to_string(),
                mime_type: "image/svg+xml".to_string(),
                annotations: None,
                meta: None,
            },
        },
    ];

    assert_eq!(complex_messages.len(), 5);

    // Verify each message type
    match &complex_messages[1].content {
        ContentBlock::ResourceLink { resource, .. } => {
            assert_eq!(resource.name, "dataset");
        }
        _ => panic!("Expected ResourceLink"),
    }

    match &complex_messages[3].content {
        ContentBlock::Resource { resource, .. } => match resource {
            ResourceContents::Text(text_contents) => assert!(text_contents.text.contains("rows")),
            _ => panic!("Expected Text resource"),
        },
        _ => panic!("Expected Resource"),
    }

    match &complex_messages[4].content {
        ContentBlock::Image { mime_type, .. } => {
            assert_eq!(mime_type, "image/svg+xml");
        }
        _ => panic!("Expected Image"),
    }
}
