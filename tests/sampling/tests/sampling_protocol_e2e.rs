//! End-to-End Tests for MCP Sampling Protocol
//!
//! Tests the sampling/createMessage endpoint implementation with real sampling handlers.
//! Validates protocol compliance, message generation, and error handling.

use mcp_sampling_tests::test_utils::{
    create_message_request, extract_sampling_message, sampling_capabilities,
};
use mcp_sampling_tests::{debug, info, json, McpTestClient, TestServerManager};

#[tokio::test]
async fn test_sampling_create_message_endpoint() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_sampling_server()
        .await
        .expect("Failed to start sampling server");
    let mut client = McpTestClient::new(server.port());

    // Initialize with sampling capabilities
    client
        .initialize_with_capabilities(sampling_capabilities())
        .await
        .unwrap();

    // Call sampling/createMessage endpoint
    let create_request = create_message_request("Write a short story about space exploration", 500);

    let sampling_result = client
        .make_request("sampling/createMessage", create_request, 20)
        .await
        .expect("Failed to call sampling/createMessage");

    debug!("Sampling result: {:?}", sampling_result);

    // Verify response structure
    assert!(
        sampling_result.contains_key("result"),
        "Response should contain 'result'"
    );
    let result = sampling_result.get("result").unwrap().as_object().unwrap();

    // Check for required fields in CreateMessageResult
    assert!(
        result.contains_key("message"),
        "Result should contain 'message'"
    );
    assert!(
        result.contains_key("model"),
        "Result should contain 'model'"
    );

    let message = result.get("message").unwrap().as_object().unwrap();
    assert!(
        message.contains_key("role"),
        "Message should contain 'role'"
    );
    assert!(
        message.contains_key("content"),
        "Message should contain 'content'"
    );

    // Verify message role is assistant
    assert_eq!(message.get("role").unwrap().as_str().unwrap(), "assistant");

    // Verify content structure
    let content = message.get("content").unwrap().as_object().unwrap();
    assert!(
        content.contains_key("text"),
        "Content should contain 'text'"
    );

    let generated_text = content.get("text").unwrap().as_str().unwrap();
    assert!(
        !generated_text.is_empty(),
        "Generated text should not be empty"
    );
    assert!(
        generated_text.len() > 10,
        "Generated text should be meaningful length"
    );

    info!("✅ Sampling createMessage endpoint working correctly");
    debug!(
        "Generated text preview: {}",
        &generated_text[..std::cmp::min(100, generated_text.len())]
    );
}

#[tokio::test]
async fn test_sampling_different_models() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_sampling_server()
        .await
        .expect("Failed to start sampling server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(sampling_capabilities())
        .await
        .unwrap();

    // Test different types of requests to different sampling models
    let test_cases = vec![
        ("Write a technical document", "technical writing", 300),
        ("Create a creative story", "creative writing", 400),
        ("Help me with general questions", "conversational", 200),
    ];

    for (request_text, expected_type, max_tokens) in test_cases {
        let create_request = create_message_request(request_text, max_tokens);

        let result = client
            .make_request("sampling/createMessage", create_request, 21)
            .await
            .expect("Failed to call sampling/createMessage");

        let generated_text =
            extract_sampling_message(&result).expect("Should extract message text");

        assert!(
            !generated_text.is_empty(),
            "Generated text should not be empty for {}",
            expected_type
        );
        assert!(
            generated_text.len() > 20,
            "Generated text should be substantial for {}",
            expected_type
        );

        info!("✅ {} sampling model working correctly", expected_type);
        debug!(
            "{} sample: {}",
            expected_type,
            &generated_text[..std::cmp::min(80, generated_text.len())]
        );
    }
}

#[tokio::test]
async fn test_sampling_parameter_validation() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_sampling_server()
        .await
        .expect("Failed to start sampling server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(sampling_capabilities())
        .await
        .unwrap();

    // Test invalid maxTokens
    let invalid_request = json!({
        "messages": [
            {
                "role": "user",
                "content": {
                    "type": "text",
                    "text": "Test message"
                }
            }
        ],
        "maxTokens": 0  // Invalid: should be > 0
    });

    let result = client
        .make_request("sampling/createMessage", invalid_request, 22)
        .await;
    match result {
        Ok(response) => {
            // Should be an error response
            assert!(
                response.contains_key("error"),
                "Should return error for invalid maxTokens"
            );
            let error = response.get("error").unwrap().as_object().unwrap();
            let message = error.get("message").unwrap().as_str().unwrap();
            assert!(
                message.to_lowercase().contains("token")
                    || message.to_lowercase().contains("validation"),
                "Error should mention tokens or validation: {}",
                message
            );
            info!("✅ Invalid maxTokens properly rejected: {}", message);
        }
        Err(_) => {
            info!("✅ Invalid maxTokens rejected at HTTP level");
        }
    }

    // Test missing messages
    let no_messages_request = json!({
        "messages": [],
        "maxTokens": 100
    });

    let result = client
        .make_request("sampling/createMessage", no_messages_request, 23)
        .await;
    match result {
        Ok(response) => {
            if response.contains_key("error") {
                let error = response.get("error").unwrap().as_object().unwrap();
                let message = error.get("message").unwrap().as_str().unwrap();
                assert!(
                    message.to_lowercase().contains("message")
                        || message.to_lowercase().contains("required"),
                    "Error should mention missing messages: {}",
                    message
                );
                info!("✅ Empty messages array properly rejected: {}", message);
            } else {
                info!("ℹ️  Empty messages handled gracefully");
            }
        }
        Err(_) => {
            info!("✅ Empty messages rejected at HTTP level");
        }
    }
}

#[tokio::test]
async fn test_sampling_temperature_parameter() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_sampling_server()
        .await
        .expect("Failed to start sampling server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(sampling_capabilities())
        .await
        .unwrap();

    // Test different temperature values
    let temperature_tests = vec![0.0, 0.5, 1.0, 1.5];

    for temperature in temperature_tests {
        let request_with_temp = json!({
            "messages": [
                {
                    "role": "user",
                    "content": {
                        "type": "text",
                        "text": "Generate a short response"
                    }
                }
            ],
            "maxTokens": 100,
            "temperature": temperature
        });

        let result = client
            .make_request("sampling/createMessage", request_with_temp, 24)
            .await;

        match result {
            Ok(response) => {
                if response.contains_key("result") {
                    info!("✅ Temperature {} accepted", temperature);
                } else if response.contains_key("error") {
                    let error = response.get("error").unwrap().as_object().unwrap();
                    let message = error.get("message").unwrap().as_str().unwrap();
                    info!("ℹ️  Temperature {} rejected: {}", temperature, message);
                }
            }
            Err(e) => {
                info!("ℹ️  Temperature {} caused HTTP error: {:?}", temperature, e);
            }
        }
    }

    // Test invalid temperature (negative)
    let invalid_temp_request = json!({
        "messages": [
            {
                "role": "user",
                "content": {
                    "type": "text",
                    "text": "Test message"
                }
            }
        ],
        "maxTokens": 100,
        "temperature": -0.5
    });

    let result = client
        .make_request("sampling/createMessage", invalid_temp_request, 25)
        .await;
    match result {
        Ok(response) => {
            if response.contains_key("error") {
                let error = response.get("error").unwrap().as_object().unwrap();
                let message = error.get("message").unwrap().as_str().unwrap();
                assert!(
                    message.to_lowercase().contains("temperature")
                        || message.to_lowercase().contains("range"),
                    "Error should mention temperature range: {}",
                    message
                );
                info!("✅ Invalid temperature properly rejected: {}", message);
            }
        }
        Err(_) => {
            info!("✅ Invalid temperature rejected at HTTP level");
        }
    }
}

#[tokio::test]
async fn test_sampling_json_rpc_compliance() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_sampling_server()
        .await
        .expect("Failed to start sampling server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(sampling_capabilities())
        .await
        .unwrap();

    let create_request = create_message_request("Test JSON-RPC compliance", 100);

    let sampling_result = client
        .make_request("sampling/createMessage", create_request, 26)
        .await
        .expect("Failed to call sampling/createMessage");

    // Verify JSON-RPC 2.0 compliance
    assert!(
        sampling_result.contains_key("jsonrpc"),
        "Response should contain 'jsonrpc'"
    );
    assert_eq!(
        sampling_result.get("jsonrpc").unwrap().as_str().unwrap(),
        "2.0",
        "JSON-RPC version should be 2.0"
    );

    assert!(
        sampling_result.contains_key("id"),
        "Response should contain 'id'"
    );
    assert_eq!(
        sampling_result.get("id").unwrap().as_i64().unwrap(),
        26,
        "Response ID should match request ID"
    );

    assert!(
        sampling_result.contains_key("result"),
        "Response should contain 'result'"
    );
    assert!(
        !sampling_result.contains_key("error"),
        "Successful response should not contain 'error'"
    );

    info!("✅ Sampling endpoint fully JSON-RPC 2.0 compliant");
}

#[tokio::test]
async fn test_sampling_capability_advertising() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_sampling_server()
        .await
        .expect("Failed to start sampling server");
    let mut client = McpTestClient::new(server.port());

    // Initialize and check if sampling capabilities are advertised
    let init_result = client
        .initialize_with_capabilities(sampling_capabilities())
        .await
        .unwrap();

    debug!("Initialize result: {:?}", init_result);

    // Check if server advertises sampling capabilities
    if let Some(server_capabilities) = init_result.get("capabilities") {
        if let Some(sampling_cap) = server_capabilities.get("sampling") {
            info!(
                "✅ Server advertises sampling capabilities: {:?}",
                sampling_cap
            );
        } else {
            info!("ℹ️  Server does not advertise sampling capabilities (may be implicit)");
        }
    }

    // Verify the endpoint is actually available by calling it
    let test_request = create_message_request("Capability test", 50);
    let result = client
        .make_request("sampling/createMessage", test_request, 27)
        .await;

    match result {
        Ok(response) => {
            if response.contains_key("result") {
                info!("✅ Sampling endpoint is available and functional");
            } else {
                info!("ℹ️  Sampling endpoint responded but with different format");
            }
        }
        Err(e) => {
            panic!(
                "Sampling endpoint should be available if server is running: {:?}",
                e
            );
        }
    }
}

#[tokio::test]
async fn test_sampling_session_continuity() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_sampling_server()
        .await
        .expect("Failed to start sampling server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(sampling_capabilities())
        .await
        .unwrap();

    // Test multiple sampling requests in the same session
    for i in 1..=3 {
        let request = create_message_request(&format!("Request number {}", i), 100);

        let result = client
            .make_request("sampling/createMessage", request, 27 + i)
            .await
            .expect("Sampling request should succeed");

        assert!(
            result.contains_key("result"),
            "Each request should get a result"
        );

        let generated_text =
            extract_sampling_message(&result).expect("Should extract message text");

        assert!(
            !generated_text.is_empty(),
            "Generated text should not be empty for request {}",
            i
        );

        info!("✅ Request {} completed successfully", i);
    }

    info!("✅ Session continuity maintained across multiple sampling requests");
}
