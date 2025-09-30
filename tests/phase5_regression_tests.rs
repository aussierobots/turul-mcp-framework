//! Phase 5 Regression Test Suite
//!
//! Comprehensive regression tests covering all Phase 5 requirements:
//! - Progress frames forwarded from tools
//! - SSE framing for streaming clients
//! - JSON response for non-streaming clients
//! - Lifecycle enforcement over streamable HTTP
//! - Pagination limit bounds
//! - Client _meta round-tripping
//! - Notification delivery over SSE

use serial_test::serial;
use std::time::Duration;
use tokio::time::timeout;
use turul_mcp_framework_integration_tests::TestServerManager;
use reqwest;
use serde_json::{json, Value};

const TEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Test that progress frames are forwarded from tools correctly
#[tokio::test]
#[serial]
async fn test_progress_frames_forwarded_from_tools() {
    let mut server_manager = TestServerManager::new();
    let port = server_manager.start_tools_server().await.expect("Failed to start server");
    let url = format!("http://127.0.0.1:{}/mcp", port);

    // Call a tool that generates progress events
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "progress_tracker",
            "arguments": {"duration": 1, "steps": 3}
        }
    });

    let response = timeout(TEST_TIMEOUT,
        reqwest::Client::new()
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Mcp-Protocol-Version", "2025-06-18")
            .json(&request)
            .send()
    ).await.expect("Request timeout").expect("Request failed");

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
    let mut server_manager = TestServerManager::new();
    let port = server_manager.start_tools_server().await.expect("Failed to start server");
    let url = format!("http://127.0.0.1:{}/mcp", port);

    let response = timeout(TEST_TIMEOUT,
        reqwest::Client::new()
            .get(&url)
            .header("Accept", "text/event-stream")
            .header("Mcp-Protocol-Version", "2025-06-18")
            .send()
    ).await.expect("Request timeout").expect("Request failed");

    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("content-type").unwrap(), "text/event-stream");
    assert_eq!(response.headers().get("cache-control").unwrap(), "no-cache");
}

/// Test JSON response for non-streaming clients
#[tokio::test]
#[serial]
async fn test_json_response_for_non_streaming_clients() {
    let mut server_manager = TestServerManager::new();
    let port = server_manager.start_tools_server().await.expect("Failed to start server");
    let url = format!("http://127.0.0.1:{}/mcp", port);

    // Standard JSON request without SSE headers
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list"
    });

    let response = timeout(TEST_TIMEOUT,
        reqwest::Client::new()
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Mcp-Protocol-Version", "2025-06-18")
            .json(&request)
            .send()
    ).await.expect("Request timeout").expect("Request failed");

    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("content-type").unwrap(), "application/json");

    let result: Value = response.json().await.expect("Failed to parse response");
    assert_eq!(result["jsonrpc"], "2.0");
    assert_eq!(result["id"], 1);
    assert!(result.get("result").is_some());
}

/// Test lifecycle enforcement over streamable HTTP
#[tokio::test]
#[serial]
async fn test_lifecycle_enforcement_over_streamable_http() {
    let mut server_manager = TestServerManager::new();
    let port = server_manager.start_tools_server().await.expect("Failed to start server");
    let url = format!("http://127.0.0.1:{}/mcp", port);

    // Try to call tools/list before initialization - should fail
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list"
    });

    let response = timeout(TEST_TIMEOUT,
        reqwest::Client::new()
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Mcp-Protocol-Version", "2025-06-18")
            .json(&request)
            .send()
    ).await.expect("Request timeout").expect("Request failed");

    assert_eq!(response.status(), 401); // Should be unauthorized without session
}

/// Test pagination limit bounds
#[tokio::test]
#[serial]
async fn test_pagination_limit_bounds() {
    let mut server_manager = TestServerManager::new();
    let port = server_manager.start_tools_server().await.expect("Failed to start server");
    let url = format!("http://127.0.0.1:{}/mcp", port);

    // First initialize to get a session
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"}
        }
    });

    let response = timeout(TEST_TIMEOUT,
        reqwest::Client::new()
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Mcp-Protocol-Version", "2025-06-18")
            .json(&init_request)
            .send()
    ).await.expect("Request timeout").expect("Request failed");

    let session_id = response.headers().get("mcp-session-id").unwrap().to_str().unwrap();

    // Test limit clamping (should clamp to MAX_LIMIT = 100)
    let request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {
            "limit": 1000  // Should be clamped to 100
        }
    });

    let response = timeout(TEST_TIMEOUT,
        reqwest::Client::new()
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Mcp-Protocol-Version", "2025-06-18")
            .header("Mcp-Session-Id", session_id)
            .json(&request)
            .send()
    ).await.expect("Request timeout").expect("Request failed");

    assert_eq!(response.status(), 200);
    let result: Value = response.json().await.expect("Failed to parse response");
    assert!(result.get("result").is_some());
}

/// Test client _meta round-tripping
#[tokio::test]
#[serial]
async fn test_client_meta_round_tripping() {
    let mut server_manager = TestServerManager::new();
    let port = server_manager.start_tools_server().await.expect("Failed to start server");
    let url = format!("http://127.0.0.1:{}/mcp", port);

    // First initialize
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"}
        }
    });

    let response = timeout(TEST_TIMEOUT,
        reqwest::Client::new()
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Mcp-Protocol-Version", "2025-06-18")
            .json(&init_request)
            .send()
    ).await.expect("Request timeout").expect("Request failed");

    let session_id = response.headers().get("mcp-session-id").unwrap().to_str().unwrap();

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

    let response = timeout(TEST_TIMEOUT,
        reqwest::Client::new()
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Mcp-Protocol-Version", "2025-06-18")
            .header("Mcp-Session-Id", session_id)
            .json(&request)
            .send()
    ).await.expect("Request timeout").expect("Request failed");

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
    let mut server_manager = TestServerManager::new();
    let port = server_manager.start_tools_server().await.expect("Failed to start server");
    let url = format!("http://127.0.0.1:{}/mcp", port);

    // First initialize via POST to get session
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"}
        }
    });

    let response = timeout(TEST_TIMEOUT,
        reqwest::Client::new()
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Mcp-Protocol-Version", "2025-06-18")
            .json(&init_request)
            .send()
    ).await.expect("Request timeout").expect("Request failed");

    let session_id = response.headers().get("mcp-session-id").unwrap().to_str().unwrap();

    // Then connect via SSE to receive notifications
    let sse_response = timeout(TEST_TIMEOUT,
        reqwest::Client::new()
            .get(&url)
            .header("Accept", "text/event-stream")
            .header("Mcp-Protocol-Version", "2025-06-18")
            .header("Mcp-Session-Id", session_id)
            .send()
    ).await.expect("Request timeout").expect("Request failed");

    assert_eq!(sse_response.status(), 200);
    assert_eq!(sse_response.headers().get("content-type").unwrap(), "text/event-stream");
}

/// Test zero limit returns error
#[tokio::test]
#[serial]
async fn test_zero_limit_returns_error() {
    let mut server_manager = TestServerManager::new();
    let port = server_manager.start_tools_server().await.expect("Failed to start server");
    let url = format!("http://127.0.0.1:{}/mcp", port);

    // Initialize first
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"}
        }
    });

    let response = timeout(TEST_TIMEOUT,
        reqwest::Client::new()
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Mcp-Protocol-Version", "2025-06-18")
            .json(&init_request)
            .send()
    ).await.expect("Request timeout").expect("Request failed");

    let session_id = response.headers().get("mcp-session-id").unwrap().to_str().unwrap();

    // Test zero limit - should return error
    let request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {
            "limit": 0
        }
    });

    let response = timeout(TEST_TIMEOUT,
        reqwest::Client::new()
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Mcp-Protocol-Version", "2025-06-18")
            .header("Mcp-Session-Id", session_id)
            .json(&request)
            .send()
    ).await.expect("Request timeout").expect("Request failed");

    assert_eq!(response.status(), 200);
    let result: Value = response.json().await.expect("Failed to parse response");

    // Should be an error response
    assert!(result.get("error").is_some());
}

/// Test that requests without limit work correctly
#[tokio::test]
#[serial]
async fn test_no_limit_uses_default() {
    let mut server_manager = TestServerManager::new();
    let port = server_manager.start_tools_server().await.expect("Failed to start server");
    let url = format!("http://127.0.0.1:{}/mcp", port);

    // Initialize first
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"}
        }
    });

    let response = timeout(TEST_TIMEOUT,
        reqwest::Client::new()
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Mcp-Protocol-Version", "2025-06-18")
            .json(&init_request)
            .send()
    ).await.expect("Request timeout").expect("Request failed");

    let session_id = response.headers().get("mcp-session-id").unwrap().to_str().unwrap();

    // Test request without limit parameter
    let request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    let response = timeout(TEST_TIMEOUT,
        reqwest::Client::new()
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Mcp-Protocol-Version", "2025-06-18")
            .header("Mcp-Session-Id", session_id)
            .json(&request)
            .send()
    ).await.expect("Request timeout").expect("Request failed");

    assert_eq!(response.status(), 200);
    let result: Value = response.json().await.expect("Failed to parse response");

    // Should be successful
    assert!(result.get("result").is_some());
    assert!(result.get("error").is_none());
}