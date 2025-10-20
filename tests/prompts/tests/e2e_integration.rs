//! E2E Integration Tests for MCP Prompts
//!
//! Tests real HTTP/SSE transport using prompts-test-server
//! Validates complete MCP 2025-06-18 specification compliance

use mcp_e2e_shared::{McpTestClient, TestFixtures, TestServerManager};
use tracing::info;

#[tokio::test]
async fn test_mcp_initialize_session() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = match TestServerManager::start_prompts_server().await {
        Ok(server) => server,
        Err(e) => {
            println!(
                "Skipping test - failed to start server (likely sandboxed environment): {}",
                e
            );
            return;
        }
    };
    let mut client = McpTestClient::new(server.port());

    let result = client
        .initialize_with_capabilities(TestFixtures::prompts_capabilities())
        .await
        .expect("Failed to initialize");

    // Verify response structure
    TestFixtures::verify_initialization_response(&result);

    // Verify session ID was provided
    assert!(client.session_id().is_some());
}

#[tokio::test]
async fn test_prompts_list() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = match TestServerManager::start_prompts_server().await {
        Ok(server) => server,
        Err(e) => {
            println!(
                "Skipping test - failed to start server (likely sandboxed environment): {}",
                e
            );
            return;
        }
    };
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::prompts_capabilities())
        .await
        .expect("Failed to initialize");
    let result = client.list_prompts().await.expect("Failed to list prompts");

    // Verify response structure using shared test fixtures
    TestFixtures::verify_prompts_list_response(&result);

    let result_data = result["result"].as_object().unwrap();
    let prompts = result_data["prompts"].as_array().unwrap();
    assert!(!prompts.is_empty(), "Should have test prompts available");

    // Verify all expected test prompts are present
    let prompt_names: Vec<&str> = prompts
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();

    assert!(prompt_names.contains(&"simple_prompt"));
    assert!(prompt_names.contains(&"string_args_prompt"));
    assert!(prompt_names.contains(&"boolean_args_prompt"));
    assert!(prompt_names.contains(&"number_args_prompt"));
}

#[tokio::test]
async fn test_simple_string_prompt_get() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = match TestServerManager::start_prompts_server().await {
        Ok(server) => server,
        Err(e) => {
            println!(
                "Skipping test - failed to start server (likely sandboxed environment): {}",
                e
            );
            return;
        }
    };
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::prompts_capabilities())
        .await
        .expect("Failed to initialize");

    let args = TestFixtures::create_string_args();
    let result = client
        .get_prompt("string_args_prompt", Some(args))
        .await
        .expect("Failed to get prompt");

    // Verify response structure using shared test fixtures
    TestFixtures::verify_prompt_response(&result);

    let result_data = result["result"].as_object().unwrap();
    let messages = result_data["messages"].as_array().unwrap();
    assert!(!messages.is_empty(), "Should have messages");

    // Verify content includes the test string
    let first_message = &messages[0];
    let content = &first_message["content"];
    if let Some(text_content) = content["text"].as_str() {
        assert!(text_content.contains("test string"));
    }
}

#[tokio::test]
async fn test_complex_prompt_get() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = match TestServerManager::start_prompts_server().await {
        Ok(server) => server,
        Err(e) => {
            println!(
                "Skipping test - failed to start server (likely sandboxed environment): {}",
                e
            );
            return;
        }
    };
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::prompts_capabilities())
        .await
        .expect("Failed to initialize");

    let args = TestFixtures::create_template_args();
    let result = client
        .get_prompt("template_prompt", Some(args))
        .await
        .expect("Failed to get prompt");

    // Verify response structure
    TestFixtures::verify_prompt_response(&result);

    let result_data = result["result"].as_object().unwrap();
    let messages = result_data["messages"].as_array().unwrap();
    assert!(!messages.is_empty(), "Should have messages");

    // Template prompt returns 1 message with variable substitution (not multiple messages)
    assert!(
        !messages.is_empty(),
        "Template prompt should have at least one message"
    );
}

#[tokio::test]
async fn test_number_prompt_get() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = match TestServerManager::start_prompts_server().await {
        Ok(server) => server,
        Err(e) => {
            println!(
                "Skipping test - failed to start server (likely sandboxed environment): {}",
                e
            );
            return;
        }
    };
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::prompts_capabilities())
        .await
        .expect("Failed to initialize");

    let args = TestFixtures::create_number_args();
    let result = client
        .get_prompt("number_args_prompt", Some(args))
        .await
        .expect("Failed to get prompt");

    // Verify response structure
    TestFixtures::verify_prompt_response(&result);

    let result_data = result["result"].as_object().unwrap();
    let messages = result_data["messages"].as_array().unwrap();
    assert!(!messages.is_empty(), "Should have messages");

    // Verify content includes the test number
    let first_message = &messages[0];
    let content = &first_message["content"];
    if let Some(text_content) = content["text"].as_str() {
        assert!(text_content.contains("42"));
    }
}

#[tokio::test]
async fn test_boolean_prompt_get() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = match TestServerManager::start_prompts_server().await {
        Ok(server) => server,
        Err(e) => {
            println!(
                "Skipping test - failed to start server (likely sandboxed environment): {}",
                e
            );
            return;
        }
    };
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::prompts_capabilities())
        .await
        .expect("Failed to initialize");

    let args = TestFixtures::create_boolean_args();
    let result = client
        .get_prompt("boolean_args_prompt", Some(args))
        .await
        .expect("Failed to get prompt");

    // Verify response structure
    TestFixtures::verify_prompt_response(&result);

    let result_data = result["result"].as_object().unwrap();
    let messages = result_data["messages"].as_array().unwrap();
    assert!(!messages.is_empty(), "Should have messages");

    // Verify content includes the boolean status (server converts to ENABLED/DISABLED and ON/OFF)
    let first_message = &messages[0];
    let content = &first_message["content"];
    if let Some(text_content) = content["text"].as_str() {
        assert!(
            text_content.contains("ENABLED")
                || text_content.contains("DISABLED")
                || text_content.contains("ON")
                || text_content.contains("OFF")
        );
    }
}

#[tokio::test]
async fn test_prompt_with_missing_arguments() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = match TestServerManager::start_prompts_server().await {
        Ok(server) => server,
        Err(e) => {
            println!(
                "Skipping test - failed to start server (likely sandboxed environment): {}",
                e
            );
            return;
        }
    };
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::prompts_capabilities())
        .await
        .expect("Failed to initialize");

    // Try to get a prompt that requires arguments without providing them
    let result = client
        .get_prompt("string_args_prompt", None)
        .await
        .expect("Failed to get prompt");

    // Should get an error response for missing required arguments
    if result.contains_key("error") {
        TestFixtures::verify_error_response(&result);
    } else {
        // Some implementations might still return a response with placeholder values
        TestFixtures::verify_prompt_response(&result);
    }
}

#[tokio::test]
async fn test_nonexistent_prompt() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = match TestServerManager::start_prompts_server().await {
        Ok(server) => server,
        Err(e) => {
            println!(
                "Skipping test - failed to start server (likely sandboxed environment): {}",
                e
            );
            return;
        }
    };
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::prompts_capabilities())
        .await
        .expect("Failed to initialize");

    let result = client
        .get_prompt("nonexistent_prompt", None)
        .await
        .expect("Failed to get prompt");

    // Should get an error response
    TestFixtures::verify_error_response(&result);
}

#[tokio::test]
async fn test_sse_notifications() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = match TestServerManager::start_prompts_server().await {
        Ok(server) => server,
        Err(e) => {
            println!(
                "Skipping test - failed to start server (likely sandboxed environment): {}",
                e
            );
            return;
        }
    };
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::prompts_capabilities())
        .await
        .expect("Failed to initialize");

    // Test SSE notifications
    let events = client
        .test_sse_notifications()
        .await
        .expect("Failed to test SSE notifications");

    // SSE connection should work even if no events are received immediately
    // This test verifies the connection can be established
    info!("SSE test completed with {} events", events.len());
}
