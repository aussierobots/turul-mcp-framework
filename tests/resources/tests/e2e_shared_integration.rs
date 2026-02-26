//! E2E Integration Tests for MCP Resources using Shared Utilities
//!
//! Tests real HTTP/SSE transport using resource-test-server with shared utilities

use mcp_e2e_shared::{McpTestClient, SessionTestUtils, TestFixtures, TestServerManager};
use serial_test::serial;
use tracing::info;

#[tokio::test]
#[serial]
async fn test_resource_initialization_with_shared_utils() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    let result = client
        .initialize_with_capabilities(TestFixtures::resource_capabilities())
        .await
        .expect("Failed to initialize");

    TestFixtures::verify_initialization_response(&result);
    assert!(client.session_id().is_some());
}

#[tokio::test]
#[serial]
async fn test_resource_list_with_shared_utils() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::resource_capabilities())
        .await
        .expect("Failed to initialize");

    let result = client
        .list_resources()
        .await
        .expect("Failed to list resources");

    TestFixtures::verify_resource_list_response(&result);

    // Verify specific resources are present
    let result_data = result["result"].as_object().unwrap();
    let resources = result_data["resources"].as_array().unwrap();
    assert!(
        !resources.is_empty(),
        "Should have test resources available"
    );

    // Check for key resource types
    let resource_uris: Vec<&str> = resources.iter().filter_map(|r| r["uri"].as_str()).collect();

    assert!(resource_uris.iter().any(|uri| uri.starts_with("file://")));
    assert!(
        resource_uris
            .iter()
            .any(|uri| uri.starts_with("file:///memory/"))
    );
    assert!(
        resource_uris
            .iter()
            .any(|uri| uri.starts_with("file:///error/"))
    );
    // Note: Template resources are returned by resources/templates/list, not resources/list
}

#[tokio::test]
#[serial]
async fn test_resource_memory_read_with_shared_utils() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    let result = client
        .read_resource("file:///memory/data.json")
        .await
        .expect("Failed to read memory resource");

    if result.contains_key("result") {
        TestFixtures::verify_resource_content_response(&result);

        let result_data = result["result"].as_object().unwrap();
        let contents = result_data["contents"].as_array().unwrap();
        let content = &contents[0];
        let content_obj = content.as_object().unwrap();
        assert_eq!(content_obj["uri"], "file:///memory/data.json");
    }
}

#[tokio::test]
#[serial]
async fn test_resource_error_handling_with_shared_utils() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    let result = client
        .read_resource("file:///error/not_found.txt")
        .await
        .expect("Request should succeed but resource should error");

    // Should get a JSON-RPC error response for not found resource
    assert!(
        result.contains_key("error"),
        "Error resource should return JSON-RPC error response"
    );
    TestFixtures::verify_error_response(&result);
}

#[tokio::test]
#[serial]
async fn test_session_consistency_resources() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    SessionTestUtils::verify_session_consistency(&client)
        .await
        .expect("Session consistency verification failed");
}

#[tokio::test]
#[serial]
async fn test_session_aware_resource_with_shared_utils() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    SessionTestUtils::test_session_aware_resource(&client)
        .await
        .expect("Session-aware resource test failed");
}

#[tokio::test]
#[serial]
async fn test_resource_subscription_with_shared_utils() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::resource_capabilities())
        .await
        .expect("Failed to initialize");

    let result = client
        .subscribe_resource("subscribe://updates")
        .await
        .expect("Failed to subscribe to resource");

    // Should get acknowledgment or successful response
    assert!(result.contains_key("result") || result.contains_key("error"));

    if result.contains_key("error") {
        // If there's an error, it shouldn't be a session or protocol error
        let error_message = result["error"]["message"].as_str().unwrap_or("");
        assert!(!error_message.to_lowercase().contains("session"));
        assert!(!error_message.to_lowercase().contains("protocol"));
    }
}

#[tokio::test]
#[serial]
async fn test_sse_notifications_resources_with_shared_utils() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::resource_capabilities())
        .await
        .expect("Failed to initialize");

    // Subscribe to notifications first
    let _subscribe_result = client.subscribe_resource("notify://trigger").await;

    // Test SSE stream
    let events = client
        .test_sse_notifications()
        .await
        .expect("Failed to test SSE notifications");

    // Should receive some SSE data format (if any events are available)
    if !events.is_empty() {
        info!("Received SSE events: {:?}", events);
        // SSE format validation - events should contain proper SSE format:
        // - "data:" for data fields
        // - "event:" for event type fields
        // - ":" for comments (keepalive, etc)
        let has_sse_format = events.iter().any(|e| {
            let trimmed = e.trim();
            !trimmed.is_empty()
                && (trimmed.contains("data:")
                    || trimmed.contains("event:")
                    || trimmed.starts_with(':'))
        });

        // Only assert if we got non-empty content
        if events.iter().any(|e| !e.trim().is_empty()) {
            assert!(
                has_sse_format,
                "Expected SSE format (data:, event:, or : comment) in non-empty events, got: {:?}",
                events
            );
        }
    }
}

#[tokio::test]
#[serial]
async fn test_multiple_resource_types_with_shared_utils() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    let test_resources = vec![
        ("file:///tmp/test.txt", "file"),
        ("file:///memory/data.json", "memory"),
        ("file:///template/items/123.json", "template"),
        ("file:///empty/content.txt", "empty"),
        ("file:///binary/image.png", "binary"),
    ];

    for (uri, resource_type) in test_resources {
        let result = client
            .read_resource(uri)
            .await
            .unwrap_or_else(|_| panic!("Failed to read {} resource", resource_type));

        if result.contains_key("result") {
            info!("Successfully read {} resource: {}", resource_type, uri);
            TestFixtures::verify_resource_content_response(&result);
        } else if result.contains_key("error") {
            info!(
                "{} resource returned error (expected for some): {}",
                resource_type, uri
            );
            TestFixtures::verify_error_response(&result);
        }
    }
}
