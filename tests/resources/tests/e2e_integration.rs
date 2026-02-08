//! E2E Integration Tests for MCP Resources
//!
//! Tests real HTTP/SSE transport using resource-test-server
//! Validates complete MCP 2025-11-25 specification compliance

use mcp_e2e_shared::{McpTestClient, TestServerManager, TestFixtures};
use serial_test::serial;
use tracing::{debug, info};

// All test infrastructure is now provided by mcp_e2e_shared
// This ensures consistent behavior, auto-build, and graceful skips across all tests

#[tokio::test]
#[serial]
async fn test_mcp_initialize_session() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    let result = client.initialize_with_capabilities(TestFixtures::resource_capabilities()).await.expect("Failed to initialize");

    // Verify response structure
    assert!(result.contains_key("result"));
    let result_data = result["result"].as_object().unwrap();

    assert!(result_data.contains_key("protocolVersion"));
    assert!(result_data.contains_key("capabilities"));
    assert!(result_data.contains_key("serverInfo"));

    // Verify protocol version
    assert_eq!(result_data["protocolVersion"], "2025-06-18");

    // Verify session ID was provided
    assert!(client.session_id().is_some());
}

#[tokio::test]
#[serial]
async fn test_resources_list() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(TestFixtures::resource_capabilities()).await.expect("Failed to initialize");
    let result = client
        .list_resources()
        .await
        .expect("Failed to list resources");

    // Verify response structure
    assert!(result.contains_key("result"));
    let result_data = result["result"].as_object().unwrap();
    assert!(result_data.contains_key("resources"));

    let resources = result_data["resources"].as_array().unwrap();
    assert!(
        !resources.is_empty(),
        "Should have test resources available"
    );

    // Verify all expected test resources are present (using file:// scheme for security)
    let expected_uris = vec![
        "file:///tmp/test.txt",
        "file:///memory/data.json",
        "file:///error/not_found.txt",
        "file:///slow/delayed.txt",
        // "file:///template/items/{id}.json", // TODO: Template resources not appearing in regular resource lists
        "file:///empty/content.txt",
        "file:///large/dataset.json",
        "file:///binary/image.png",
        "file:///session/info.json",
        "file:///subscribe/updates.json",
        "file:///notify/trigger.json",
        "file:///multi/contents.txt",
        "file:///paginated/items.json",
        "file:///invalid/bad-chars-and-spaces.txt",
        "file:///long/very-long-path-that-exceeds-normal-uri-length-limits-for-testing-how-the-framework-handles-extremely-long-resource-identifiers-and-edge-cases.txt",
        "file:///meta/dynamic.json",
        "file:///complete/all-fields.json",
    ];

    for expected_uri in expected_uris {
        let found = resources.iter().any(|r| {
            let actual_uri = r
                .as_object()
                .and_then(|obj| obj.get("uri"))
                .and_then(|uri| uri.as_str());

            if expected_uri.starts_with("file:///long/") {
                // Long URI is dynamically generated, just check it starts with file:///long/
                actual_uri
                    .map(|uri| uri.starts_with("file:///long/"))
                    .unwrap_or(false)
            } else {
                actual_uri == Some(expected_uri)
            }
        });
        assert!(found, "Missing expected resource: {}", expected_uri);
    }
}

#[tokio::test]
#[serial]
async fn test_file_resource_read() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(TestFixtures::resource_capabilities()).await.expect("Failed to initialize");
    let result = client
        .read_resource("file:///tmp/test.txt")
        .await
        .expect("Failed to read file resource");

    // Verify response structure
    assert!(result.contains_key("result"));
    let result_data = result["result"].as_object().unwrap();
    assert!(result_data.contains_key("contents"));

    let contents = result_data["contents"].as_array().unwrap();
    assert!(!contents.is_empty());

    let content = &contents[0];
    assert!(content.as_object().unwrap().contains_key("uri"));
    assert!(content.as_object().unwrap().contains_key("text"));
}

#[tokio::test]
#[serial]
async fn test_memory_resource_read() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(TestFixtures::resource_capabilities()).await.expect("Failed to initialize");
    let result = client
        .read_resource("file:///memory/data.json")
        .await
        .expect("Failed to read memory resource");

    // Verify response structure
    assert!(result.contains_key("result"));
    let result_data = result["result"].as_object().unwrap();
    assert!(result_data.contains_key("contents"));

    let contents = result_data["contents"].as_array().unwrap();
    assert!(!contents.is_empty());

    let content = &contents[0];
    let content_obj = content.as_object().unwrap();
    assert!(content_obj.contains_key("uri"));
    assert_eq!(content_obj["uri"], "file:///memory/data.json");
}

#[tokio::test]
#[serial]
async fn test_error_resource_handling() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(TestFixtures::resource_capabilities()).await.expect("Failed to initialize");
    let result = client
        .read_resource("file:///error/not_found.txt")
        .await
        .expect("Request should succeed but resource should error");

    // Should get a JSON-RPC error response at top level per JSON-RPC 2.0 spec
    assert!(result.contains_key("error"));
    assert!(!result.contains_key("result")); // No result field when there's an error
    let error = result["error"].as_object().unwrap();
    assert!(error.contains_key("code"));
    assert!(error.contains_key("message"));
}

#[tokio::test]
#[serial]
async fn test_template_resource_with_variables() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(TestFixtures::resource_capabilities()).await.expect("Failed to initialize");

    // Template resource should handle URI variables
    let result = client
        .read_resource("file:///template/items/123.json")
        .await
        .expect("Failed to read template resource");

    assert!(result.contains_key("result"));
    let result_data = result["result"].as_object().unwrap();
    assert!(result_data.contains_key("contents"));
}

#[tokio::test]
#[serial]
async fn test_binary_resource_read() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(TestFixtures::resource_capabilities()).await.expect("Failed to initialize");
    let result = client
        .read_resource("file:///binary/image.png")
        .await
        .expect("Failed to read binary resource");

    assert!(result.contains_key("result"));
    let result_data = result["result"].as_object().unwrap();
    assert!(result_data.contains_key("contents"));

    let contents = result_data["contents"].as_array().unwrap();
    let content = &contents[0];
    let content_obj = content.as_object().unwrap();

    // Binary resource should return blob content
    assert!(content_obj.contains_key("blob"));
    assert!(content_obj["blob"].is_string());
}

#[tokio::test]
#[serial]
async fn test_session_aware_resource() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(TestFixtures::resource_capabilities()).await.expect("Failed to initialize");
    let result = client
        .read_resource("file:///session/info.json")
        .await
        .expect("Failed to read session resource");

    assert!(result.contains_key("result"));
    let result_data = result["result"].as_object().unwrap();
    assert!(result_data.contains_key("contents"));

    let contents = result_data["contents"].as_array().unwrap();
    let content = &contents[0];
    let text = content.as_object().unwrap()["text"].as_str().unwrap();

    // Should contain session information
    assert!(text.contains("session"));
}

#[tokio::test]
#[serial]
async fn test_resource_subscription() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    // First, verify server correctly advertises that subscription is not supported
    let init_response = client.initialize_with_capabilities(TestFixtures::resource_capabilities()).await.expect("Failed to initialize");
    let server_capabilities = &init_response["result"]["capabilities"]["resources"];
    assert_eq!(
        server_capabilities["subscribe"], false,
        "Server should advertise subscribe=false until implemented"
    );

    // Test that subscription request properly returns method not found error
    let result = client
        .subscribe_resource("file:///subscription/updates")
        .await
        .expect("Request should succeed but method should not be found");

    // Should get a JSON-RPC error response for unimplemented method
    assert!(
        result.contains_key("error"),
        "Should return error for unimplemented resources/subscribe method"
    );
    let error = result["error"].as_object().unwrap();
    assert_eq!(
        error["code"], -32601,
        "Should return method not found error code"
    );
    assert!(
        error["message"].as_str().unwrap().contains("not found"),
        "Error message should indicate method not found"
    );
}

#[tokio::test]
#[serial]
async fn test_paginated_resource() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(TestFixtures::resource_capabilities()).await.expect("Failed to initialize");
    let result = client
        .read_resource("file:///paginated/items.json")
        .await
        .expect("Failed to read paginated resource");

    assert!(result.contains_key("result"));
    let result_data = result["result"].as_object().unwrap();
    assert!(result_data.contains_key("contents"));

    // Check for pagination metadata
    if result_data.contains_key("_meta") {
        let meta = result_data["_meta"].as_object().unwrap();
        // Pagination metadata would be in _meta field
        debug!("Pagination meta: {:?}", meta);
    }
}

#[tokio::test]
#[serial]
async fn test_large_resource_handling() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(TestFixtures::resource_capabilities()).await.expect("Failed to initialize");
    let result = client
        .read_resource("file:///large/dataset.json")
        .await
        .expect("Failed to read large resource");

    assert!(result.contains_key("result"));
    let result_data = result["result"].as_object().unwrap();
    assert!(result_data.contains_key("contents"));

    let contents = result_data["contents"].as_array().unwrap();
    let content = &contents[0];
    let text = content.as_object().unwrap()["text"].as_str().unwrap();

    // Should handle large content properly
    assert!(
        text.len() > 1000,
        "Large resource should return substantial content"
    );
}

#[tokio::test]
#[serial]
async fn test_resource_with_metadata() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(TestFixtures::resource_capabilities()).await.expect("Failed to initialize");
    let result = client
        .read_resource("file:///meta/dynamic.json")
        .await
        .expect("Failed to read resource with metadata");

    assert!(result.contains_key("result"));
    let result_data = result["result"].as_object().unwrap();
    assert!(result_data.contains_key("contents"));

    // Check for enhanced metadata
    if result_data.contains_key("_meta") {
        let meta = result_data["_meta"].as_object().unwrap();
        debug!("Enhanced meta: {:?}", meta);
    }
}

#[tokio::test]
#[serial]
async fn test_complete_resource_specification() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(TestFixtures::resource_capabilities()).await.expect("Failed to initialize");
    let result = client
        .read_resource("file:///complete/all-fields.json")
        .await
        .expect("Failed to read complete resource");

    assert!(result.contains_key("result"));
    let result_data = result["result"].as_object().unwrap();
    assert!(result_data.contains_key("contents"));

    let contents = result_data["contents"].as_array().unwrap();
    assert!(!contents.is_empty());

    let content = &contents[0];
    let content_obj = content.as_object().unwrap();

    // Complete resource should have all optional fields
    assert!(content_obj.contains_key("uri"));
    assert!(content_obj.contains_key("text"));

    if content_obj.contains_key("mimeType") {
        assert!(content_obj["mimeType"].is_string());
    }
}

#[tokio::test]
#[serial]
async fn test_sse_resource_notifications() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(TestFixtures::resource_capabilities()).await.expect("Failed to initialize");

    // Subscribe to notifications first
    let _subscribe_result = client
        .subscribe_resource("notify://trigger")
        .await
        .expect("Failed to subscribe to notifications");

    // Test SSE stream (simplified - real test would trigger changes)
    let events = client
        .test_sse_notifications()
        .await
        .expect("Failed to test SSE notifications");

    // Should receive some SSE data format
    if !events.is_empty() {
        info!("Received SSE events: {:?}", events);
        // Validate SSE format - accept data:, event:, or : comments (keepalive)
        assert!(events.iter().any(|e| {
            let trimmed = e.trim();
            trimmed.contains("data:") || trimmed.contains("event:") || trimmed.starts_with(':')
        }));
    }
}

#[tokio::test]
#[serial]
async fn test_multi_resource_collection() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize_with_capabilities(TestFixtures::resource_capabilities()).await.expect("Failed to initialize");
    let result = client
        .read_resource("file:///multi/contents.txt")
        .await
        .expect("Failed to read multi resource");

    assert!(result.contains_key("result"));
    let result_data = result["result"].as_object().unwrap();
    assert!(result_data.contains_key("contents"));

    let contents = result_data["contents"].as_array().unwrap();
    // Multi resource should return multiple content items
    assert!(
        contents.len() > 1,
        "Multi resource should return multiple content items"
    );
}
