//! Content-Type negotiation tests for MCP streamable HTTP transport.
//!
//! Verifies that the server's response Content-Type matches the client's Accept header
//! and that the response body format is consistent with the declared Content-Type.
//!
//! Per MCP spec, servers may return either `application/json` or `text/event-stream`
//! depending on the Accept header. These tests assert wire-format consistency.

use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use turul_mcp_server::McpServer;
use turul_mcp_session_storage::InMemorySessionStorage;

async fn start_negotiation_test_server() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server_url = format!("http://127.0.0.1:{}/mcp", addr.port());
    drop(listener);

    use turul_mcp_derive::mcp_tool;
    use turul_mcp_protocol::McpResult;

    #[mcp_tool(name = "echo", description = "Echo tool for content-type tests")]
    async fn echo(message: String) -> McpResult<String> {
        Ok(message)
    }

    let session_storage = Arc::new(InMemorySessionStorage::new());
    let server = McpServer::builder()
        .name("content-type-negotiation-test")
        .version("1.0.0")
        .tool_fn(echo)
        .with_session_storage(session_storage)
        .bind_address(addr)
        .build()
        .unwrap();

    tokio::spawn(async move {
        if let Err(e) = server.run().await {
            eprintln!("Server error: {}", e);
        }
    });

    sleep(Duration::from_millis(200)).await;
    server_url
}

/// Complete the MCP handshake and return the session ID.
async fn initialize_session(client: &reqwest::Client, server_url: &str, accept: &str) -> String {
    let init_response = client
        .post(server_url)
        .header("Content-Type", "application/json")
        .header("Accept", accept)
        .header("MCP-Protocol-Version", "2025-11-25")
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": { "name": "content-type-test", "version": "1.0.0" }
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(init_response.status(), 200, "initialize should succeed");

    let session_id = init_response
        .headers()
        .get("Mcp-Session-Id")
        .expect("Server must return session ID")
        .to_str()
        .unwrap()
        .to_string();

    // Complete handshake
    let initialized_response = client
        .post(server_url)
        .header("Content-Type", "application/json")
        .header("Accept", accept)
        .header("MCP-Protocol-Version", "2025-11-25")
        .header("Mcp-Session-Id", &session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }))
        .send()
        .await
        .unwrap();

    assert!(
        initialized_response.status() == 200 || initialized_response.status() == 202,
        "notifications/initialized should succeed: got {}",
        initialized_response.status()
    );

    session_id
}

/// Parse a JSON-RPC response body, asserting format matches Content-Type.
/// Returns the parsed JSON value.
fn parse_response_body(content_type: &str, body: &str) -> Value {
    if content_type.contains("text/event-stream") {
        // SSE: must have "data: " prefixed lines containing JSON-RPC objects
        let json_str = body
            .lines()
            .find_map(|line| line.strip_prefix("data: "))
            .unwrap_or_else(|| panic!("SSE response must contain data: lines, got: {}", body));
        serde_json::from_str(json_str)
            .unwrap_or_else(|e| panic!("SSE data line must be valid JSON: {} — body: {}", e, body))
    } else if content_type.contains("application/json") {
        // JSON: body is a raw JSON-RPC object
        serde_json::from_str(body).unwrap_or_else(|e| {
            panic!(
                "application/json body must be valid JSON: {} — body: {}",
                e, body
            )
        })
    } else {
        panic!(
            "Unexpected Content-Type '{}' — expected text/event-stream or application/json",
            content_type
        );
    }
}

/// POST with `Accept: application/json` only — server MUST respond with `application/json`.
#[tokio::test]
async fn test_tools_call_json_only_accept_returns_json() {
    let server_url = start_negotiation_test_server().await;
    let client = reqwest::Client::new();
    let accept = "application/json";
    let session_id = initialize_session(&client, &server_url, accept).await;

    let response = client
        .post(&server_url)
        .header("Content-Type", "application/json")
        .header("Accept", accept)
        .header("MCP-Protocol-Version", "2025-11-25")
        .header("Mcp-Session-Id", &session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "id": 2,
            "params": {
                "name": "echo",
                "arguments": { "message": "hello" }
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    assert!(
        content_type.contains("application/json"),
        "Accept: application/json must produce Content-Type: application/json, got '{}'",
        content_type
    );

    let body = response.text().await.unwrap();
    let parsed = parse_response_body(&content_type, &body);
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert!(
        parsed.get("result").is_some(),
        "tools/call should return a result"
    );
}

/// POST with `Accept: text/event-stream` — server MUST respond with `text/event-stream`.
#[tokio::test]
async fn test_tools_call_sse_only_accept_returns_sse() {
    let server_url = start_negotiation_test_server().await;
    let client = reqwest::Client::new();
    let accept = "text/event-stream";
    let session_id = initialize_session(&client, &server_url, accept).await;

    let response = client
        .post(&server_url)
        .header("Content-Type", "application/json")
        .header("Accept", accept)
        .header("MCP-Protocol-Version", "2025-11-25")
        .header("Mcp-Session-Id", &session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "id": 2,
            "params": {
                "name": "echo",
                "arguments": { "message": "hello" }
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    assert!(
        content_type.contains("text/event-stream"),
        "Accept: text/event-stream must produce Content-Type: text/event-stream, got '{}'",
        content_type
    );

    let body = response.text().await.unwrap();
    let parsed = parse_response_body(&content_type, &body);
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert!(
        parsed.get("result").is_some(),
        "tools/call should return a result"
    );
}

/// Combined Accept + `tools/call` → SSE (conservative heuristic).
///
/// Any tool _might_ call `notify_progress()` mid-stream, and the transport layer
/// cannot know at Content-Type decision time whether the specific tool will.
/// So `tools/call` always gets SSE under combined Accept — even for simple tools
/// that return a single result with no progress events.
#[tokio::test]
async fn test_tools_call_combined_accept_prefers_sse() {
    let server_url = start_negotiation_test_server().await;
    let client = reqwest::Client::new();
    let accept = "application/json, text/event-stream";
    let session_id = initialize_session(&client, &server_url, accept).await;

    let response = client
        .post(&server_url)
        .header("Content-Type", "application/json")
        .header("Accept", accept)
        .header("MCP-Protocol-Version", "2025-11-25")
        .header("Mcp-Session-Id", &session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "id": 2,
            "params": {
                "name": "echo",
                "arguments": { "message": "hello" }
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    // tools/call is streaming-capable — server should use SSE even with combined Accept
    assert!(
        content_type.contains("text/event-stream"),
        "tools/call with combined Accept should prefer SSE, got '{}'",
        content_type
    );

    let body = response.text().await.unwrap();
    let parsed = parse_response_body(&content_type, &body);
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert!(
        parsed.get("result").is_some(),
        "tools/call should return a result"
    );
}

/// Combined Accept + `tools/list` (non-streaming method) → JSON.
///
/// Transport policy heuristic: methods that never emit progress notifications
/// prefer `application/json` when the client accepts both formats.
#[tokio::test]
async fn test_tools_list_combined_accept_prefers_json() {
    let server_url = start_negotiation_test_server().await;
    let client = reqwest::Client::new();
    let accept = "application/json, text/event-stream";
    let session_id = initialize_session(&client, &server_url, accept).await;

    let response = client
        .post(&server_url)
        .header("Content-Type", "application/json")
        .header("Accept", accept)
        .header("MCP-Protocol-Version", "2025-11-25")
        .header("Mcp-Session-Id", &session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": 2
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    // tools/list never streams — server should prefer JSON with combined Accept
    assert!(
        content_type.contains("application/json"),
        "tools/list with combined Accept should prefer JSON, got '{}'",
        content_type
    );

    let body = response.text().await.unwrap();
    let parsed = parse_response_body(&content_type, &body);
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert!(
        parsed["result"]["tools"].is_array(),
        "tools/list should return tools array"
    );
}
