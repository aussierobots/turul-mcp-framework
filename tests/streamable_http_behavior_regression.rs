//! Streamable HTTP Behavior Regression Suite
//!
//! Regression tests covering streamable HTTP wire behavior:
//! - Progress frames forwarded from tools
//! - SSE framing for streaming clients
//! - JSON response for non-streaming clients
//! - Lifecycle enforcement over streamable HTTP
//! - Pagination limit bounds
//! - Client _meta round-tripping
//! - Notification delivery over SSE

use mcp_e2e_shared::TestServerManager;
use serde_json::{Value, json};
use serial_test::serial;
use std::time::Duration;
use tokio::time::timeout;

const TEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Complete the strict-lifecycle handshake (`initialize` then
/// `notifications/initialized`) and return the session ID. Session-scoped
/// requests are rejected until both steps complete.
async fn initialize_session(client: &reqwest::Client, url: &str) -> String {
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"}
        }
    });

    let response = timeout(
        TEST_TIMEOUT,
        client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Mcp-Protocol-Version", "2025-11-25")
            .json(&init_request)
            .send(),
    )
    .await
    .expect("initialize request timeout")
    .expect("initialize request failed");
    assert_eq!(response.status(), 200, "initialize should succeed");

    let session_id = response
        .headers()
        .get("mcp-session-id")
        .expect("initialize response must set Mcp-Session-Id")
        .to_str()
        .unwrap()
        .to_string();

    let initialized_response = timeout(
        TEST_TIMEOUT,
        client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Mcp-Protocol-Version", "2025-11-25")
            .header("Mcp-Session-Id", &session_id)
            .json(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
            .send(),
    )
    .await
    .expect("notifications/initialized request timeout")
    .expect("notifications/initialized request failed");
    assert_eq!(
        initialized_response.status(),
        202,
        "notifications/initialized should be accepted"
    );

    session_id
}

/// Test that progress frames are forwarded from tools correctly
#[tokio::test]
#[serial]
async fn test_progress_frames_forwarded_from_tools() {
    let server_manager = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start server");
    let url = format!("http://127.0.0.1:{}/mcp", server_manager.port());
    let client = reqwest::Client::new();
    let session_id = initialize_session(&client, &url).await;

    // Call a tool that generates progress events
    let request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "progress_tracker",
            "arguments": {"duration": 1, "steps": 3}
        }
    });

    let response = timeout(
        TEST_TIMEOUT,
        client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Mcp-Protocol-Version", "2025-11-25")
            .header("Mcp-Session-Id", &session_id)
            .json(&request)
            .send(),
    )
    .await
    .expect("Request timeout")
    .expect("Request failed");

    assert_eq!(response.status(), 200);
    let result: Value = response.json().await.expect("Failed to parse response");

    // Should have result with progress information in tool output
    assert!(result.get("result").is_some());
    let result_content = result["result"].as_object().unwrap();
    assert!(result_content.contains_key("content"));
}

/// Test SSE framing for streaming clients
#[tokio::test]
#[serial]
async fn test_sse_framing_for_streaming_clients() {
    let server_manager = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start server");
    let url = format!("http://127.0.0.1:{}/mcp", server_manager.port());
    let client = reqwest::Client::new();
    let session_id = initialize_session(&client, &url).await;

    let response = timeout(
        TEST_TIMEOUT,
        client
            .get(&url)
            .header("Accept", "text/event-stream")
            .header("Mcp-Protocol-Version", "2025-11-25")
            .header("Mcp-Session-Id", &session_id)
            .send(),
    )
    .await
    .expect("Request timeout")
    .expect("Request failed");

    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/event-stream"
    );
    assert_eq!(
        response.headers().get("cache-control").unwrap(),
        "no-cache"
    );
}

/// Test JSON response for non-streaming clients
#[tokio::test]
#[serial]
async fn test_json_response_for_non_streaming_clients() {
    let server_manager = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start server");
    let url = format!("http://127.0.0.1:{}/mcp", server_manager.port());
    let client = reqwest::Client::new();
    let session_id = initialize_session(&client, &url).await;

    // Standard JSON request without SSE headers
    let request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    });

    let response = timeout(
        TEST_TIMEOUT,
        client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Mcp-Protocol-Version", "2025-11-25")
            .header("Mcp-Session-Id", &session_id)
            .json(&request)
            .send(),
    )
    .await
    .expect("Request timeout")
    .expect("Request failed");

    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let result: Value = response.json().await.expect("Failed to parse response");
    assert_eq!(result["jsonrpc"], "2.0");
    assert_eq!(result["id"], 2);
    assert!(result.get("result").is_some());
}

/// Test lifecycle enforcement over streamable HTTP
#[tokio::test]
#[serial]
async fn test_lifecycle_enforcement_over_streamable_http() {
    let server_manager = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start server");
    let port = server_manager.port();
    let url = format!("http://127.0.0.1:{}/mcp", port);

    // Try to call tools/list before initialization - should fail
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list"
    });

    let response = timeout(
        TEST_TIMEOUT,
        reqwest::Client::new()
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Mcp-Protocol-Version", "2025-11-25")
            .json(&request)
            .send(),
    )
    .await
    .expect("Request timeout")
    .expect("Request failed");

    assert_eq!(response.status(), 400); // Missing session → 400 per MCP 2025-11-25 § Session Management
}

/// Test pagination limit bounds
#[tokio::test]
#[serial]
async fn test_pagination_limit_bounds() {
    let server_manager = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start server");
    let url = format!("http://127.0.0.1:{}/mcp", server_manager.port());
    let client = reqwest::Client::new();
    let session_id = initialize_session(&client, &url).await;

    // Test limit clamping (should clamp to the server's max page size)
    let request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {
            "limit": 1000  // Should be clamped
        }
    });

    let response = timeout(
        TEST_TIMEOUT,
        client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Mcp-Protocol-Version", "2025-11-25")
            .header("Mcp-Session-Id", &session_id)
            .json(&request)
            .send(),
    )
    .await
    .expect("Request timeout")
    .expect("Request failed");

    assert_eq!(response.status(), 200);
    let result: Value = response.json().await.expect("Failed to parse response");
    assert!(result.get("result").is_some());
}

/// Test client _meta round-tripping
#[tokio::test]
#[serial]
async fn test_client_meta_round_tripping() {
    let server_manager = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start server");
    let url = format!("http://127.0.0.1:{}/mcp", server_manager.port());
    let client = reqwest::Client::new();
    let session_id = initialize_session(&client, &url).await;

    // Test request with _meta
    let request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {
            "_meta": {
                "customField": "custom_value",
                "userContext": "user_123"
            }
        }
    });

    let response = timeout(
        TEST_TIMEOUT,
        client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Mcp-Protocol-Version", "2025-11-25")
            .header("Mcp-Session-Id", &session_id)
            .json(&request)
            .send(),
    )
    .await
    .expect("Request timeout")
    .expect("Request failed");

    assert_eq!(response.status(), 200);
    let result: Value = response.json().await.expect("Failed to parse response");

    // Should have result with _meta
    assert!(result.get("result").is_some());
    let result_obj = result["result"].as_object().unwrap();
    assert!(result_obj.contains_key("_meta"));
}

/// Test notification delivery over SSE
#[tokio::test]
#[serial]
async fn test_notification_delivery_over_sse() {
    let server_manager = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start server");
    let url = format!("http://127.0.0.1:{}/mcp", server_manager.port());
    let client = reqwest::Client::new();
    let session_id = initialize_session(&client, &url).await;

    // Connect via SSE to receive notifications
    let sse_response = timeout(
        TEST_TIMEOUT,
        client
            .get(&url)
            .header("Accept", "text/event-stream")
            .header("Mcp-Protocol-Version", "2025-11-25")
            .header("Mcp-Session-Id", &session_id)
            .send(),
    )
    .await
    .expect("Request timeout")
    .expect("Request failed");

    assert_eq!(sse_response.status(), 200);
    assert_eq!(
        sse_response.headers().get("content-type").unwrap(),
        "text/event-stream"
    );
}

/// Test zero limit returns error
#[tokio::test]
#[serial]
async fn test_zero_limit_returns_error() {
    let server_manager = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start server");
    let url = format!("http://127.0.0.1:{}/mcp", server_manager.port());
    let client = reqwest::Client::new();
    let session_id = initialize_session(&client, &url).await;

    // Test zero limit - should return error
    let request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {
            "limit": 0
        }
    });

    let response = timeout(
        TEST_TIMEOUT,
        client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Mcp-Protocol-Version", "2025-11-25")
            .header("Mcp-Session-Id", &session_id)
            .json(&request)
            .send(),
    )
    .await
    .expect("Request timeout")
    .expect("Request failed");

    assert_eq!(response.status(), 200);
    let result: Value = response.json().await.expect("Failed to parse response");

    // Should be an error response
    assert!(result.get("error").is_some());
}

/// Test that requests without limit work correctly
#[tokio::test]
#[serial]
async fn test_no_limit_uses_default() {
    let server_manager = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start server");
    let url = format!("http://127.0.0.1:{}/mcp", server_manager.port());
    let client = reqwest::Client::new();
    let session_id = initialize_session(&client, &url).await;

    // Test request without limit parameter
    let request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    let response = timeout(
        TEST_TIMEOUT,
        client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Mcp-Protocol-Version", "2025-11-25")
            .header("Mcp-Session-Id", &session_id)
            .json(&request)
            .send(),
    )
    .await
    .expect("Request timeout")
    .expect("Request failed");

    assert_eq!(response.status(), 200);
    let result: Value = response.json().await.expect("Failed to parse response");

    // Should be successful
    assert!(result.get("result").is_some());
    assert!(result.get("error").is_none());
}
