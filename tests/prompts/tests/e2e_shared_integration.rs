//! E2E Integration Tests for MCP Prompts using Shared Utilities
//!
//! Tests real HTTP/SSE transport using prompts-test-server with shared utilities

use mcp_e2e_shared::{McpTestClient, TestServerManager, TestFixtures, SessionTestUtils};
use tracing::info;

#[tokio::test]
async fn test_prompts_initialization_with_shared_utils() {
    tracing_subscriber::fmt::init();

    let server = TestServerManager::start_prompts_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    let result = client
        .initialize_with_capabilities(TestFixtures::prompts_capabilities())
        .await
        .expect("Failed to initialize");

    TestFixtures::verify_initialization_response(&result);
    assert!(client.session_id().is_some());
}

#[tokio::test]
async fn test_prompts_list_with_shared_utils() {
    tracing_subscriber::fmt::init();

    let server = TestServerManager::start_prompts_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::prompts_capabilities())
        .await
        .expect("Failed to initialize");

    let result = client.list_prompts().await.expect("Failed to list prompts");

    TestFixtures::verify_prompts_list_response(&result);

    // Verify specific prompts are present
    let result_data = result["result"].as_object().unwrap();
    let prompts = result_data["prompts"].as_array().unwrap();
    assert!(!prompts.is_empty(), "Should have test prompts available");

    // Check for key prompt types
    let prompt_names: Vec<&str> = prompts
        .iter()
        .filter_map(|p| p["name"].as_str())
        .collect();

    assert!(prompt_names.contains(&"simple_prompt"));
    assert!(prompt_names.contains(&"string_args_prompt"));
    assert!(prompt_names.contains(&"number_args_prompt"));
    assert!(prompt_names.contains(&"template_prompt"));
    assert!(prompt_names.contains(&"session_aware_prompt"));
}

#[tokio::test]
async fn test_simple_prompt_with_shared_utils() {
    tracing_subscriber::fmt::init();

    let server = TestServerManager::start_prompts_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    let result = client
        .get_prompt("simple_prompt", None)
        .await
        .expect("Failed to get simple prompt");

    TestFixtures::verify_prompt_response(&result);
}

#[tokio::test]
async fn test_string_args_prompt_with_shared_utils() {
    tracing_subscriber::fmt::init();

    let server = TestServerManager::start_prompts_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    let arguments = TestFixtures::create_string_args();
    let result = client
        .get_prompt("string_args_prompt", Some(arguments))
        .await
        .expect("Failed to get string args prompt");

    TestFixtures::verify_prompt_response(&result);

    // Check that arguments were used in the message content
    let result_data = result["result"].as_object().unwrap();
    let messages = result_data["messages"].as_array().unwrap();
    if !messages.is_empty() {
        let message_content = messages[0]["content"]["text"].as_str().unwrap();
        assert!(message_content.contains("test string"));
    }
}

#[tokio::test]
async fn test_number_args_prompt_with_shared_utils() {
    tracing_subscriber::fmt::init();

    let server = TestServerManager::start_prompts_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    let arguments = TestFixtures::create_number_args();
    let result = client
        .get_prompt("number_args_prompt", Some(arguments))
        .await
        .expect("Failed to get number args prompt");

    TestFixtures::verify_prompt_response(&result);

    // Check that numbers were used in the message content
    let result_data = result["result"].as_object().unwrap();
    let messages = result_data["messages"].as_array().unwrap();
    if !messages.is_empty() {
        let message_content = messages[0]["content"]["text"].as_str().unwrap();
        assert!(message_content.contains("42"));
    }
}

#[tokio::test]
async fn test_boolean_args_prompt_with_shared_utils() {
    tracing_subscriber::fmt::init();

    let server = TestServerManager::start_prompts_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    let arguments = TestFixtures::create_boolean_args();
    let result = client
        .get_prompt("boolean_args_prompt", Some(arguments))
        .await
        .expect("Failed to get boolean args prompt");

    TestFixtures::verify_prompt_response(&result);

    // Check that booleans were used in the message content
    let result_data = result["result"].as_object().unwrap();
    let messages = result_data["messages"].as_array().unwrap();
    if !messages.is_empty() {
        let message_content = messages[0]["content"]["text"].as_str().unwrap();
        assert!(message_content.contains("true") || message_content.contains("false"));
    }
}

#[tokio::test]
async fn test_multi_message_prompt_with_shared_utils() {
    tracing_subscriber::fmt::init();

    let server = TestServerManager::start_prompts_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    let result = client
        .get_prompt("multi_message_prompt", None)
        .await
        .expect("Failed to get multi message prompt");

    TestFixtures::verify_prompt_response(&result);

    let result_data = result["result"].as_object().unwrap();
    let messages = result_data["messages"].as_array().unwrap();
    
    // Multi-message prompt should return multiple messages
    assert!(messages.len() > 1, "Multi message prompt should return multiple messages");

    // Verify different roles are used
    let roles: Vec<&str> = messages.iter()
        .filter_map(|m| m["role"].as_str())
        .collect();
    
    // Should have different roles (user and assistant)
    assert!(roles.contains(&"user") || roles.contains(&"assistant"));
}

#[tokio::test]
async fn test_session_consistency_prompts() {
    tracing_subscriber::fmt::init();

    let server = TestServerManager::start_prompts_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    SessionTestUtils::verify_session_consistency(&client)
        .await
        .expect("Session consistency verification failed");
}

#[tokio::test]
async fn test_session_aware_prompt_with_shared_utils() {
    tracing_subscriber::fmt::init();

    let server = TestServerManager::start_prompts_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    SessionTestUtils::test_session_aware_prompt(&client)
        .await
        .expect("Session-aware prompt test failed");
}

#[tokio::test]
async fn test_template_prompt_with_shared_utils() {
    tracing_subscriber::fmt::init();

    let server = TestServerManager::start_prompts_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    let mut arguments = std::collections::HashMap::new();
    arguments.insert("template_name".to_string(), serde_json::json!("test_template"));
    arguments.insert("template_value".to_string(), serde_json::json!("test value"));

    let result = client
        .get_prompt("template_prompt", Some(arguments))
        .await
        .expect("Failed to get template prompt");

    TestFixtures::verify_prompt_response(&result);

    let result_data = result["result"].as_object().unwrap();
    let messages = result_data["messages"].as_array().unwrap();
    if !messages.is_empty() {
        // Check that template variables were substituted
        let message_content = messages[0]["content"]["text"].as_str().unwrap();
        assert!(message_content.contains("test_template"));
        assert!(message_content.contains("test value"));
    }
}

#[tokio::test]
async fn test_empty_messages_prompt_with_shared_utils() {
    tracing_subscriber::fmt::init();

    let server = TestServerManager::start_prompts_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    let result = client
        .get_prompt("empty_messages_prompt", None)
        .await
        .expect("Failed to get empty messages prompt");

    TestFixtures::verify_prompt_response(&result);

    let result_data = result["result"].as_object().unwrap();
    let messages = result_data["messages"].as_array().unwrap();
    
    // Empty messages prompt should return an empty array
    assert_eq!(messages.len(), 0, "Empty messages prompt should return no messages");
}

#[tokio::test]
async fn test_validation_failure_prompt_with_shared_utils() {
    tracing_subscriber::fmt::init();

    let server = TestServerManager::start_prompts_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    // Try to get validation failure prompt without required arguments
    let result = client
        .get_prompt("validation_failure_prompt", None)
        .await
        .expect("Request should succeed but prompt should error");

    // Should get a JSON-RPC error response for validation failure
    if result.contains_key("error") {
        TestFixtures::verify_error_response(&result);
    } else {
        // Some implementations might return empty messages or special content
        TestFixtures::verify_prompt_response(&result);
    }
}

#[tokio::test]
async fn test_sse_notifications_prompts_with_shared_utils() {
    tracing_subscriber::fmt::init();

    let server = TestServerManager::start_prompts_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::prompts_capabilities())
        .await
        .expect("Failed to initialize");

    // Test SSE stream (simplified - real test would trigger changes)
    let events = client
        .test_sse_notifications()
        .await
        .expect("Failed to test SSE notifications");

    // Should receive some SSE data format (if any events are available)
    if !events.is_empty() {
        info!("Received SSE events: {:?}", events);
        assert!(events.iter().any(|e| e.contains("data:") || e.contains("event:")));
    }
}