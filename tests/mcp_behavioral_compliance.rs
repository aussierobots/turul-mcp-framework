//! MCP Behavioral Compliance Tests
//!
//! Tests to verify that the MCP implementation correctly handles:
//! 1. _meta field propagation without overwriting pagination metadata
//! 2. Cursor-based pagination for tools/list
//! 3. Limit parameter support
//! 4. All list handlers preserve pagination fields

use reqwest;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use turul_http_mcp_server::HttpMcpServer;
use turul_mcp_session_storage::InMemorySessionStorage;

async fn start_test_server_with_tools() -> String {
    // Find an available port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server_url = format!("http://127.0.0.1:{}/mcp", addr.port());
    drop(listener);

    let session_storage = Arc::new(InMemorySessionStorage::new());
    let server = HttpMcpServer::builder_with_storage(session_storage)
        .bind_address(addr)
        .build();

    // Start server in background
    tokio::spawn(async move {
        if let Err(e) = server.run().await {
            eprintln!("Test server error: {}", e);
        }
    });

    // Wait for server to start
    sleep(Duration::from_millis(200)).await;
    server_url
}

async fn initialize_session(client: &reqwest::Client, server_url: &str) -> String {
    let init_response = client
        .post(server_url)
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2025-06-18")
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "experimental": {},
                    "sampling": {}
                },
                "clientInfo": {
                    "name": "behavioral-compliance-test",
                    "version": "1.0.0"
                }
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(init_response.status(), 200);

    init_response
        .headers()
        .get("Mcp-Session-Id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn test_meta_propagation_preserves_pagination() {
    let server_url = start_test_server_with_tools().await;
    let client = reqwest::Client::new();
    let session_id = initialize_session(&client, &server_url).await;

    // Test tools/list with request _meta
    let request_meta = json!({
        "client_context": "test-context",
        "request_id": "12345"
    });

    let response = client
        .post(&server_url)
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2025-06-18")
        .header("Mcp-Session-Id", &session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": 2,
            "params": {
                "_meta": request_meta
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: Value = response.json().await.unwrap();
    println!("Response body: {}", serde_json::to_string_pretty(&body).unwrap());

    // Verify result structure
    assert!(body["result"]["tools"].is_array());

    // Verify _meta field exists and contains both pagination AND request data
    let response_meta = &body["result"]["_meta"];
    assert!(response_meta.is_object());

    // Check that pagination fields are preserved
    assert!(response_meta["total"].is_number());
    assert!(response_meta["hasMore"].is_boolean());

    // Check that request _meta was merged into extra fields
    assert_eq!(response_meta["client_context"], "test-context");
    assert_eq!(response_meta["request_id"], "12345");
}

#[tokio::test]
async fn test_tools_list_pagination() {
    let server_url = start_test_server_with_tools().await;
    let client = reqwest::Client::new();
    let session_id = initialize_session(&client, &server_url).await;

    // Test with small limit to force pagination
    let response = client
        .post(&server_url)
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2025-06-18")
        .header("Mcp-Session-Id", &session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": 3,
            "params": {
                "limit": 1
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: Value = response.json().await.unwrap();

    // Verify pagination structure
    assert!(body["result"]["tools"].is_array());

    // If there are tools, check pagination metadata
    if body["result"]["tools"].as_array().unwrap().len() > 0 {
        assert!(body["result"]["_meta"]["total"].is_number());
        assert!(body["result"]["_meta"]["hasMore"].is_boolean());

        // With limit=1, should have at most 1 tool
        assert!(body["result"]["tools"].as_array().unwrap().len() <= 1);
    }
}

#[tokio::test]
async fn test_cursor_based_navigation() {
    let server_url = start_test_server_with_tools().await;
    let client = reqwest::Client::new();
    let session_id = initialize_session(&client, &server_url).await;

    // First request with limit
    let response1 = client
        .post(&server_url)
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2025-06-18")
        .header("Mcp-Session-Id", &session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": 4,
            "params": {
                "limit": 1
            }
        }))
        .send()
        .await
        .unwrap();

    let body1: Value = response1.json().await.unwrap();

    // If there's a nextCursor, test navigation
    if let Some(next_cursor) = body1["result"]["nextCursor"].as_str() {
        let response2 = client
            .post(&server_url)
            .header("Content-Type", "application/json")
            .header("MCP-Protocol-Version", "2025-06-18")
            .header("Mcp-Session-Id", &session_id)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "tools/list",
                "id": 5,
                "params": {
                    "cursor": next_cursor,
                    "limit": 1
                }
            }))
            .send()
            .await
            .unwrap();

        let body2: Value = response2.json().await.unwrap();

        // Should get different tools (or empty if at end)
        let tools1 = body1["result"]["tools"].as_array().unwrap();
        let tools2 = body2["result"]["tools"].as_array().unwrap();

        if !tools1.is_empty() && !tools2.is_empty() {
            // Tools should be different
            assert_ne!(tools1[0]["name"], tools2[0]["name"]);
        }
    }
}

#[tokio::test]
async fn test_limit_parameter_honored() {
    let server_url = start_test_server_with_tools().await;
    let client = reqwest::Client::new();
    let session_id = initialize_session(&client, &server_url).await;

    // Test different limit values
    for limit in [1, 5, 10] {
        let response = client
            .post(&server_url)
            .header("Content-Type", "application/json")
            .header("MCP-Protocol-Version", "2025-06-18")
            .header("Mcp-Session-Id", &session_id)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "tools/list",
                "id": 6 + limit,
                "params": {
                    "limit": limit
                }
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);

        let body: Value = response.json().await.unwrap();
        let tools = body["result"]["tools"].as_array().unwrap();

        // Should return at most 'limit' tools
        assert!(tools.len() <= limit as usize);
    }
}

#[tokio::test]
async fn test_all_handlers_meta_merge() {
    let server_url = start_test_server_with_tools().await;
    let client = reqwest::Client::new();
    let session_id = initialize_session(&client, &server_url).await;

    let test_meta = json!({
        "test_field": "test_value",
        "batch_id": "batch-123"
    });

    // Test all list endpoints
    let endpoints = ["tools/list", "resources/list", "prompts/list"];

    for endpoint in &endpoints {
        let response = client
            .post(&server_url)
            .header("Content-Type", "application/json")
            .header("MCP-Protocol-Version", "2025-06-18")
            .header("Mcp-Session-Id", &session_id)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": endpoint,
                "id": format!("test-{}", endpoint.replace("/", "-")),
                "params": {
                    "_meta": test_meta
                }
            }))
            .send()
            .await
            .unwrap();

        if response.status() == 200 {
            let body: Value = response.json().await.unwrap();

            // Verify _meta field exists
            let response_meta = &body["result"]["_meta"];
            assert!(response_meta.is_object(),
                "Endpoint {} should have _meta field", endpoint);

            // Check that request _meta was merged
            assert_eq!(response_meta["test_field"], "test_value",
                "Endpoint {} should preserve request _meta", endpoint);
            assert_eq!(response_meta["batch_id"], "batch-123",
                "Endpoint {} should preserve request _meta", endpoint);

            // Check pagination fields are still present
            if response_meta["total"].is_number() {
                assert!(response_meta["hasMore"].is_boolean(),
                    "Endpoint {} should preserve pagination metadata", endpoint);
            }
        }
    }
}

#[tokio::test]
async fn test_no_meta_request_still_has_pagination() {
    let server_url = start_test_server_with_tools().await;
    let client = reqwest::Client::new();
    let session_id = initialize_session(&client, &server_url).await;

    // Request without _meta should still have pagination metadata
    let response = client
        .post(&server_url)
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2025-06-18")
        .header("Mcp-Session-Id", &session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": 10,
            "params": {}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: Value = response.json().await.unwrap();

    // Should still have pagination metadata in _meta
    let response_meta = &body["result"]["_meta"];
    if response_meta.is_object() {
        assert!(response_meta["total"].is_number());
        assert!(response_meta["hasMore"].is_boolean());
    }
}

#[tokio::test]
async fn test_zero_limit_returns_error() {
    let server_url = start_test_server_with_tools().await;
    let client = reqwest::Client::new();
    let session_id = initialize_session(&client, &server_url).await;

    // Test limit = 0 should return error
    let response = client
        .post(&server_url)
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2025-06-18")
        .header("Mcp-Session-Id", &session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": 100,
            "params": {
                "limit": 0
            }
        }))
        .send()
        .await
        .unwrap();

    // Should return error for invalid limit
    assert_eq!(response.status(), 200); // JSON-RPC errors return 200 with error object

    let body: Value = response.json().await.unwrap();

    // Should have error field, not result
    assert!(body["error"].is_object());
    assert!(body["result"].is_null() || !body.as_object().unwrap().contains_key("result"));

    // Error message should mention limit validation
    let error_message = body["error"]["message"].as_str().unwrap();
    assert!(error_message.contains("limit"));
    assert!(error_message.contains("positive"));
}

#[tokio::test]
async fn test_limit_boundary_values() {
    let server_url = start_test_server_with_tools().await;
    let client = reqwest::Client::new();
    let session_id = initialize_session(&client, &server_url).await;

    // Test limit = 1 should work (minimum valid value)
    let response = client
        .post(&server_url)
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2025-06-18")
        .header("Mcp-Session-Id", &session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": 101,
            "params": {
                "limit": 1
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: Value = response.json().await.unwrap();

    // Should have result, not error
    assert!(body["result"].is_object());
    assert!(!body.as_object().unwrap().contains_key("error") || body["error"].is_null());

    // Should return at most 1 tool
    let tools = body["result"]["tools"].as_array().unwrap();
    assert!(tools.len() <= 1);
}