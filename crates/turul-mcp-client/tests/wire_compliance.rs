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

    let transport =
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

    let transport =
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

    let transport =
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

    let transport = HttpTransport::new(&format!("{}/mcp", mock_server.uri())).unwrap();
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

    let transport = HttpTransport::new(&format!("{}/mcp", mock_server.uri())).unwrap();
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

    let transport = HttpTransport::new(&format!("{}/mcp", mock_server.uri())).unwrap();
    transport.connect().await.unwrap();

    let _ = transport.send_request(ping_request()).await;
}

// ---------------------------------------------------------------------------
// Auth-header override (v0.3.44): rotating the bearer on a live transport.
//
// Regression net for the v0.3.43 observation that an OAuth `client_credentials`
// rotation could leave `disconnect()`'s DELETE flying under the *old* bearer
// (which the AS or upstream authorizer may have already revoked, surfacing as
// HTTP 403 Forbidden in ~15 ms). The fix exposes
// `Transport::update_auth_header()` / `McpClient::set_bearer()` so callers can
// rotate the bearer *before* DELETE without rebuilding the transport (which
// would drop the connection pool).
//
// These are wire-layer tests per CLAUDE.md rule 3: they assert what reqwest
// actually puts on the wire, not framework-internal state.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_send_delete_uses_overridden_bearer_after_rotation() {
    let mock_server = MockServer::start().await;

    // The DELETE under the NEW bearer is the one we expect.
    Mock::given(method("DELETE"))
        .and(header("Authorization", "Bearer NEW"))
        .and(header("Mcp-Session-Id", "sess-abc"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Build the transport with the OLD bearer baked into `default_headers` —
    // the exact construction-time-only state v0.3.43 had no API to update.
    let mut headers = std::collections::HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer OLD".to_string());
    let config = ConnectionConfig {
        headers: Some(headers),
        ..Default::default()
    };
    let transport =
        HttpTransport::with_config(&format!("{}/mcp", mock_server.uri()), &config).unwrap();
    transport.connect().await.unwrap();

    // Rotate the bearer before disconnect — this is the new API.
    transport
        .update_auth_header(Some("Bearer NEW".to_string()))
        .await;

    transport.send_delete("sess-abc").await.unwrap();

    // wiremock verifies expect(1) with the NEW-bearer matcher on drop.
}

#[tokio::test]
async fn test_send_request_uses_overridden_bearer_after_rotation() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(header("Authorization", "Bearer NEW"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(json_rpc_ok()),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let mut headers = std::collections::HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer OLD".to_string());
    let config = ConnectionConfig {
        headers: Some(headers),
        ..Default::default()
    };
    let transport =
        HttpTransport::with_config(&format!("{}/mcp", mock_server.uri()), &config).unwrap();
    transport.connect().await.unwrap();

    transport
        .update_auth_header(Some("Bearer NEW".to_string()))
        .await;

    let _ = transport.send_request(ping_request()).await;
}

#[tokio::test]
async fn test_clearing_override_falls_back_to_default_headers() {
    let mock_server = MockServer::start().await;

    // After clearing, default_headers should reassert (Bearer OLD).
    Mock::given(method("POST"))
        .and(header("Authorization", "Bearer OLD"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(json_rpc_ok()),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let mut headers = std::collections::HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer OLD".to_string());
    let config = ConnectionConfig {
        headers: Some(headers),
        ..Default::default()
    };
    let transport =
        HttpTransport::with_config(&format!("{}/mcp", mock_server.uri()), &config).unwrap();
    transport.connect().await.unwrap();

    // Set, then clear — the next POST must carry the original default header.
    transport
        .update_auth_header(Some("Bearer NEW".to_string()))
        .await;
    transport.update_auth_header(None).await;

    let _ = transport.send_request(ping_request()).await;
}
