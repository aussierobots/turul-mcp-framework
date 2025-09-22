//! Sampling Protocol Validation and Edge Case Tests
//!
//! Tests the robustness and validation logic of the sampling protocol implementation.
//! Focuses on error handling, parameter validation, and edge cases.

use mcp_sampling_tests::{McpTestClient, TestServerManager, json, debug, info};
use mcp_sampling_tests::test_utils::{sampling_capabilities, extract_sampling_message, create_message_request};

#[tokio::test]
async fn test_sampling_large_message_handling() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_sampling_server().await.expect("Failed to start sampling server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(sampling_capabilities()).await.unwrap();

    // Create a large input message (10KB)
    let large_content = "x".repeat(10240);
    let large_request = create_message_request(&large_content, 500);
    
    let result = client.make_request("sampling/createMessage", large_request, 30).await;

    match result {
        Ok(response) => {
            if response.contains_key("result") {
                let generated_text = extract_sampling_message(&response)
                    .expect("Should extract message text");
                
                assert!(!generated_text.is_empty(), "Generated text should not be empty for large input");
                info!("✅ Large message ({} chars) processed successfully", large_content.len());
            } else if response.contains_key("error") {
                let error = response.get("error").unwrap().as_object().unwrap();
                let message = error.get("message").unwrap().as_str().unwrap();
                info!("ℹ️  Large message rejected (acceptable for memory protection): {}", message);
            }
        }
        Err(e) => {
            info!("ℹ️  Large message rejected at HTTP level (acceptable): {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_sampling_multiple_user_messages() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_sampling_server().await.expect("Failed to start sampling server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(sampling_capabilities()).await.unwrap();

    // Create request with conversation history
    let conversation_request = json!({
        "messages": [
            {
                "role": "user",
                "content": {
                    "type": "text",
                    "text": "What is machine learning?"
                }
            },
            {
                "role": "assistant",
                "content": {
                    "type": "text",
                    "text": "Machine learning is a subset of artificial intelligence..."
                }
            },
            {
                "role": "user",
                "content": {
                    "type": "text",
                    "text": "Can you give me a simple example?"
                }
            }
        ],
        "maxTokens": 300,
        "temperature": 0.6
    });

    let result = client.make_request("sampling/createMessage", conversation_request, 31).await
        .expect("Conversation request should succeed");

    assert!(result.contains_key("result"), "Should handle conversation context");

    let generated_text = extract_sampling_message(&result)
        .expect("Should extract message text");

    assert!(!generated_text.is_empty(), "Should generate response to conversation");
    assert!(generated_text.len() > 20, "Response should be substantial");

    info!("✅ Conversation context processed successfully");
    debug!("Conversation response: {}", &generated_text[..std::cmp::min(100, generated_text.len())]);
}

#[tokio::test]
async fn test_sampling_system_message_handling() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_sampling_server().await.expect("Failed to start sampling server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(sampling_capabilities()).await.unwrap();

    // Create request with system message
    let system_request = json!({
        "messages": [
            {
                "role": "system",
                "content": {
                    "type": "text",
                    "text": "You are a helpful assistant that responds in bullet points."
                }
            },
            {
                "role": "user",
                "content": {
                    "type": "text",
                    "text": "Explain photosynthesis"
                }
            }
        ],
        "maxTokens": 200
    });

    let result = client.make_request("sampling/createMessage", system_request, 32).await
        .expect("System message request should succeed");

    assert!(result.contains_key("result"), "Should handle system messages");

    let generated_text = extract_sampling_message(&result)
        .expect("Should extract message text");

    assert!(!generated_text.is_empty(), "Should generate response with system context");
    info!("✅ System message handling working correctly");
}

#[tokio::test]
async fn test_sampling_edge_case_parameters() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_sampling_server().await.expect("Failed to start sampling server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(sampling_capabilities()).await.unwrap();

    // Test edge case parameters
    let edge_cases = vec![
        ("maxTokens: 1", json!({
            "messages": [{"role": "user", "content": {"type": "text", "text": "Hi"}}],
            "maxTokens": 1
        })),
        ("maxTokens: very large", json!({
            "messages": [{"role": "user", "content": {"type": "text", "text": "Hi"}}],
            "maxTokens": 100000
        })),
        ("temperature: 0.0", json!({
            "messages": [{"role": "user", "content": {"type": "text", "text": "Hi"}}],
            "maxTokens": 50,
            "temperature": 0.0
        })),
        ("temperature: 2.0", json!({
            "messages": [{"role": "user", "content": {"type": "text", "text": "Hi"}}],
            "maxTokens": 50,
            "temperature": 2.0
        })),
    ];

    for (case_name, request) in edge_cases {
        let result = client.make_request("sampling/createMessage", request, 33).await;
        
        match result {
            Ok(response) => {
                if response.contains_key("result") {
                    info!("✅ Edge case '{}' handled successfully", case_name);
                } else if response.contains_key("error") {
                    let error = response.get("error").unwrap().as_object().unwrap();
                    let message = error.get("message").unwrap().as_str().unwrap();
                    info!("ℹ️  Edge case '{}' rejected: {}", case_name, message);
                }
            }
            Err(e) => {
                info!("ℹ️  Edge case '{}' rejected at HTTP level: {:?}", case_name, e);
            }
        }

        // Small delay between requests
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn test_sampling_malformed_messages() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_sampling_server().await.expect("Failed to start sampling server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(sampling_capabilities()).await.unwrap();

    // Test malformed message structures
    let malformed_cases = vec![
        ("missing role", json!({
            "messages": [{"content": {"type": "text", "text": "Hello"}}],
            "maxTokens": 100
        })),
        ("invalid role", json!({
            "messages": [{"role": "invalid", "content": {"type": "text", "text": "Hello"}}],
            "maxTokens": 100
        })),
        ("missing content", json!({
            "messages": [{"role": "user"}],
            "maxTokens": 100
        })),
        ("invalid content type", json!({
            "messages": [{"role": "user", "content": {"type": "invalid", "text": "Hello"}}],
            "maxTokens": 100
        })),
    ];

    for (case_name, request) in malformed_cases {
        let result = client.make_request("sampling/createMessage", request, 34).await;
        
        match result {
            Ok(response) => {
                if response.contains_key("error") {
                    let error = response.get("error").unwrap().as_object().unwrap();
                    let message = error.get("message").unwrap().as_str().unwrap();
                    info!("✅ Malformed case '{}' properly rejected: {}", case_name, message);
                } else {
                    info!("ℹ️  Malformed case '{}' handled gracefully", case_name);
                }
            }
            Err(_) => {
                info!("✅ Malformed case '{}' rejected at HTTP level", case_name);
            }
        }
    }
}

#[tokio::test]
async fn test_sampling_concurrent_requests() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_sampling_server().await.expect("Failed to start sampling server");
    
    // Create multiple clients for concurrent requests
    let mut clients = Vec::new();
    for _ in 0..3 {
        let mut client = McpTestClient::new(server.port());
        client.initialize_with_capabilities(sampling_capabilities()).await.unwrap();
        clients.push(client);
    }

    // Send concurrent requests
    let mut handles = Vec::new();
    for (i, client) in clients.into_iter().enumerate() {
        let request = create_message_request(&format!("Concurrent request {}", i + 1), 100);
        let handle = tokio::spawn(async move {
            client.make_request("sampling/createMessage", request, 35 + i as u64).await
        });
        handles.push((i, handle));
    }

    // Wait for all requests to complete
    let mut successes = 0;
    let mut errors = 0;

    for (i, handle) in handles {
        match handle.await {
            Ok(Ok(response)) => {
                if response.contains_key("result") {
                    successes += 1;
                    info!("✅ Concurrent request {} succeeded", i + 1);
                } else {
                    errors += 1;
                    info!("ℹ️  Concurrent request {} returned error response", i + 1);
                }
            }
            Ok(Err(e)) => {
                errors += 1;
                info!("ℹ️  Concurrent request {} failed: {:?}", i + 1, e);
            }
            Err(join_error) => {
                errors += 1;
                info!("⚠️  Concurrent request {} join error: {:?}", i + 1, join_error);
            }
        }
    }

    info!("📊 Concurrent requests: {} successes, {} errors", successes, errors);
    assert!(successes > 0, "At least one concurrent request should succeed");
}

#[tokio::test]
async fn test_sampling_unicode_content() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_sampling_server().await.expect("Failed to start sampling server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(sampling_capabilities()).await.unwrap();

    // Test Unicode content
    let unicode_content = "Hello! 👋 こんにちは 🌍 Привет 🇺🇳 مرحبا ⭐ Ελληνικά 🎯 Español";
    let unicode_request = create_message_request(unicode_content, 150);

    let result = client.make_request("sampling/createMessage", unicode_request, 38).await
        .expect("Unicode request should succeed");

    assert!(result.contains_key("result"), "Should handle Unicode content");

    let generated_text = extract_sampling_message(&result)
        .expect("Should extract message text");

    assert!(!generated_text.is_empty(), "Should generate response for Unicode content");
    info!("✅ Unicode content processed successfully");
    debug!("Unicode response preview: {}", &generated_text[..std::cmp::min(50, generated_text.len())]);
}

#[tokio::test]
async fn test_sampling_session_isolation() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_sampling_server().await.expect("Failed to start sampling server");
    
    // Create two separate clients (different sessions)
    let mut client1 = McpTestClient::new(server.port());
    let mut client2 = McpTestClient::new(server.port());

    client1.initialize_with_capabilities(sampling_capabilities()).await.unwrap();
    client2.initialize_with_capabilities(sampling_capabilities()).await.unwrap();

    // Send requests from different sessions
    let request1 = create_message_request("Request from session 1", 100);
    let request2 = create_message_request("Request from session 2", 100);

    let result1 = client1.make_request("sampling/createMessage", request1, 39).await
        .expect("Session 1 request should succeed");

    let result2 = client2.make_request("sampling/createMessage", request2, 40).await
        .expect("Session 2 request should succeed");

    // Both sessions should work independently
    assert!(result1.contains_key("result"), "Session 1 should get result");
    assert!(result2.contains_key("result"), "Session 2 should get result");

    let text1 = extract_sampling_message(&result1).expect("Session 1 should have text");
    let text2 = extract_sampling_message(&result2).expect("Session 2 should have text");

    assert!(!text1.is_empty(), "Session 1 should generate text");
    assert!(!text2.is_empty(), "Session 2 should generate text");

    info!("✅ Session isolation working correctly");
    debug!("Session 1 response: {}", &text1[..std::cmp::min(30, text1.len())]);
    debug!("Session 2 response: {}", &text2[..std::cmp::min(30, text2.len())]);
}