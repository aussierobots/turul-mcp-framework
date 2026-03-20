//! Wire-level protocol compliance tests using wiremock.
//!
//! These tests verify that the HTTP transport sends the correct headers
//! on the wire. They use `HttpTransport` directly (not `McpClient`) to
//! avoid needing session initialization.

use wiremock::matchers::{header, headers, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

use turul_mcp_client::config::ConnectionConfig;
use turul_mcp_client::transport::Transport;
use turul_mcp_client::transport::http::HttpTransport;

fn json_rpc_ok() -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0", "id": "req_0", "result": {}
    })
}

fn ping_request() -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0", "id": "req_0", "method": "ping", "params": {}
    })
}

#[tokio::test]
async fn test_custom_headers_appear_on_outbound_requests() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(header("X-Custom", "test-value"))
        .and(header("Authorization", "Bearer tok"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(json_rpc_ok()),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let mut headers = std::collections::HashMap::new();
    headers.insert("X-Custom".to_string(), "test-value".to_string());
    headers.insert("Authorization".to_string(), "Bearer tok".to_string());

    let config = ConnectionConfig {
        headers: Some(headers),
        ..Default::default()
    };

    let mut transport =
        HttpTransport::with_config(&format!("{}/mcp", mock_server.uri()), &config).unwrap();
    transport.connect().await.unwrap();

    let _ = transport.send_request(ping_request()).await;
    // wiremock verifies header matchers via expect(1)
}

#[tokio::test]
async fn test_custom_user_agent() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(header("User-Agent", "my-app/2.0"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(json_rpc_ok()),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = ConnectionConfig {
        user_agent: Some("my-app/2.0".to_string()),
        ..Default::default()
    };

    let mut transport =
        HttpTransport::with_config(&format!("{}/mcp", mock_server.uri()), &config).unwrap();
    transport.connect().await.unwrap();

    let _ = transport.send_request(ping_request()).await;
}

#[tokio::test]
async fn test_no_redirects_policy() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(302).insert_header("Location", "http://example.com/redirect"),
        )
        .mount(&mock_server)
        .await;

    let config = ConnectionConfig {
        follow_redirects: false,
        ..Default::default()
    };

    let mut transport =
        HttpTransport::with_config(&format!("{}/mcp", mock_server.uri()), &config).unwrap();
    transport.connect().await.unwrap();

    let result = transport.send_request(ping_request()).await;
    assert!(
        result.is_err(),
        "302 should not be followed when redirects disabled"
    );
}

#[tokio::test]
async fn test_accept_header_on_post_requests() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(headers(
            "Accept",
            vec!["application/json", "text/event-stream"],
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(json_rpc_ok()),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let mut transport = HttpTransport::new(&format!("{}/mcp", mock_server.uri())).unwrap();
    transport.connect().await.unwrap();

    let _ = transport.send_request(ping_request()).await;
}

#[tokio::test]
async fn test_notification_post_includes_accept_header() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(headers(
            "Accept",
            vec!["application/json", "text/event-stream"],
        ))
        .respond_with(ResponseTemplate::new(202))
        .expect(1)
        .mount(&mock_server)
        .await;

    let mut transport = HttpTransport::new(&format!("{}/mcp", mock_server.uri())).unwrap();
    transport.connect().await.unwrap();

    let notification = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
        "params": {}
    });
    let _ = transport.send_notification(notification).await;
    // wiremock expect(1) will fail if Accept header is missing
}

#[tokio::test]
async fn test_mcp_protocol_version_header_on_requests() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(header("MCP-Protocol-Version", "2025-11-25"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(json_rpc_ok()),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let mut transport = HttpTransport::new(&format!("{}/mcp", mock_server.uri())).unwrap();
    transport.connect().await.unwrap();

    let _ = transport.send_request(ping_request()).await;
}
