//! Integration tests for MCP 2025-06-18 session ID compliance
//!
//! Verifies that:
//! - Only `initialize` works without session ID
//! - All other methods return 401 without session ID
//! - Session ID is properly passed through the handshake

use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use turul_mcp_server::McpServer;
use turul_mcp_session_storage::InMemorySessionStorage;

async fn start_test_server() -> String {
    // Find an available port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server_url = format!("http://127.0.0.1:{}/mcp", addr.port());
    drop(listener);

    use turul_mcp_derive::mcp_tool;
    use turul_mcp_protocol::McpResult;

    // Create a test tool for session compliance testing
    #[mcp_tool(name = "test_tool", description = "Test tool for session compliance")]
    async fn test_tool() -> McpResult<String> {
        Ok("test result".to_string())
    }

    let session_storage = Arc::new(InMemorySessionStorage::new());
    let server = McpServer::builder()
        .name("session-compliance-test-server")
        .version("1.0.0")
        .tool_fn(test_tool)
        .with_session_storage(session_storage)
        .bind_address(addr)
        .build()
        .unwrap();

    // Start server in background
    tokio::spawn(async move {
        if let Err(e) = server.run().await {
            eprintln!("Server error: {}", e);
        }
    });

    // Wait for server to start
    sleep(Duration::from_millis(200)).await;

    server_url
}

#[tokio::test]
async fn test_tools_list_without_session_returns_401() {
    let server_url = start_test_server().await;
    let client = reqwest::Client::new();

    // Try tools/list WITHOUT session ID - should return 401
    let response = client
        .post(&server_url)
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2025-06-18")
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": 1
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 401);

    let body: Value = response.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32001);
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Missing Mcp-Session-Id header")
    );
}

#[tokio::test]
async fn test_resources_list_without_session_returns_401() {
    let server_url = start_test_server().await;
    let client = reqwest::Client::new();

    // Try resources/list WITHOUT session ID - should return 401
    let response = client
        .post(&server_url)
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2025-06-18")
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "resources/list",
            "id": 1
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 401);

    let body: Value = response.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32001);
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Missing Mcp-Session-Id header")
    );
}

#[tokio::test]
async fn test_initialize_without_session_creates_session() {
    let server_url = start_test_server().await;
    let client = reqwest::Client::new();

    // Initialize WITHOUT session ID should work and return session ID
    let response = client
        .post(&server_url)
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
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    // Should return session ID in header
    let session_id = response.headers().get("Mcp-Session-Id");

    assert!(session_id.is_some());
    assert!(!session_id.unwrap().to_str().unwrap().is_empty());
}

#[tokio::test]
async fn test_discovery_methods_with_session_succeed() {
    let server_url = start_test_server().await;
    let client = reqwest::Client::new();

    // First, initialize to get session ID
    let init_response = client
        .post(&server_url)
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
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(init_response.status(), 200);

    let session_id = init_response
        .headers()
        .get("Mcp-Session-Id")
        .unwrap()
        .to_str()
        .unwrap();

    // Now tools/list WITH session ID should succeed
    let tools_response = client
        .post(&server_url)
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2025-06-18")
        .header("Mcp-Session-Id", session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": 2
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(tools_response.status(), 200);

    // Should contain result, not error
    let body_text = tools_response.text().await.unwrap();
    // We don't check the exact result since it's streamed, just that we get a response
    // The key test is that we get 200 OK instead of 401 Unauthorized
    assert!(!body_text.is_empty(), "Response should not be empty");
}

#[tokio::test]
async fn test_complete_session_flow() {
    let server_url = start_test_server().await;
    let client = reqwest::Client::new();

    // Step 1: tools/list without session should fail with 401
    let no_session_response = client
        .post(&server_url)
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2025-06-18")
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": 1
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(no_session_response.status(), 401);

    // Step 2: Initialize to get session
    let init_response = client
        .post(&server_url)
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2025-06-18")
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 2,
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "experimental": {},
                    "sampling": {}
                },
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(init_response.status(), 200);

    let session_id = init_response
        .headers()
        .get("Mcp-Session-Id")
        .unwrap()
        .to_str()
        .unwrap();

    // Step 3: tools/list with session should succeed
    let with_session_response = client
        .post(&server_url)
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2025-06-18")
        .header("Mcp-Session-Id", session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": 3
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(with_session_response.status(), 200);
}

#[tokio::test]
async fn test_mcp_inspector_flow_with_combined_accept_header() {
    let server_url = start_test_server().await;
    let client = reqwest::Client::new();

    // Test the exact flow that MCP Inspector uses:
    // 1. Initialize with combined Accept header (application/json, text/event-stream)
    // 2. Should succeed even though wants_sse_stream = true but no session ID yet
    let init_response = client
        .post(&server_url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream") // MCP Inspector header
        .header("MCP-Protocol-Version", "2025-06-18")
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {
                    "name": "mcp-inspector",
                    "version": "1.0.0"
                }
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(
        init_response.status(),
        200,
        "Initialize should succeed with combined Accept header"
    );

    // Extract session ID from response header
    let session_id = init_response
        .headers()
        .get("Mcp-Session-Id")
        .expect("Server should return session ID in header")
        .to_str()
        .unwrap();

    println!("Session ID created: {}", session_id);

    // 2. Now use that session ID for subsequent requests
    let tools_response = client
        .post(&server_url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream") // Test MCP Inspector header
        .header("MCP-Protocol-Version", "2025-06-18")
        .header("Mcp-Session-Id", session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": 2
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(
        tools_response.status(),
        200,
        "tools/list should succeed with session ID"
    );

    // Handle SSE response format (data: prefix)
    let response_text = tools_response.text().await.unwrap();
    println!("Response body: {}", response_text);

    // Parse SSE format: "data: {...json...}"
    let json_content = response_text
        .strip_prefix("data: ")
        .unwrap_or(&response_text);

    let tools_body: Value = serde_json::from_str(json_content).unwrap();
    assert_eq!(tools_body["jsonrpc"], "2.0");
    assert!(tools_body["result"]["tools"].is_array());

    println!("✅ MCP Inspector flow test passed");
}
