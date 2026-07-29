//! When the client opens the GET SSE stream.
//!
//! 2026-07-28 removed the GET stream from Streamable HTTP (a conformant server
//! answers 405), so a 2026 connection must never issue one. 2025-11-25 keeps it,
//! and the GET must carry the session id the handshake produced — which means
//! the listener has to start after negotiation, not before it.
#![cfg(feature = "client-bilingual")]

use turul_mcp_client::config::ClientConfig;
use turul_mcp_client::transport::http::HttpTransport;
use turul_mcp_client::{McpClient, McpVersion};
use wiremock::matchers::{body_partial_json, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// The GET the 2026 endpoint would answer 405, as a real server does.
async fn mount_sse_405(server: &MockServer) {
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(405))
        .mount(server)
        .await;
}

async fn mount_discover_2026(server: &MockServer) {
    Mock::given(method("POST"))
        .and(body_partial_json(
            serde_json::json!({"method": "server/discover"}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0", "id": "req_0",
                    "result": {
                        "resultType": "complete", "ttlMs": 0, "cacheScope": "public",
                        "supportedVersions": ["2026-07-28"], "capabilities": {},
                        "_meta": { "io.modelcontextprotocol/serverInfo": { "name": "mock-2026", "version": "1.0.0" } }
                    }
                })),
        )
        .mount(server)
        .await;
}

async fn mount_2025_handshake(server: &MockServer) {
    Mock::given(method("POST"))
        .and(body_partial_json(
            serde_json::json!({"method": "server/discover"}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0", "id": "req_0",
                    "error": { "code": -32601, "message": "Method not found" }
                })),
        )
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(body_partial_json(
            serde_json::json!({"method": "initialize"}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .insert_header("Mcp-Session-Id", "sess-listener")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0", "id": "req_0",
                    "result": {
                        "protocolVersion": "2025-11-25",
                        "capabilities": { "tools": { "listChanged": false } },
                        "serverInfo": { "name": "mock-2025", "version": "1.0.0" }
                    }
                })),
        )
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(body_partial_json(
            serde_json::json!({"method": "notifications/initialized"}),
        ))
        .respond_with(ResponseTemplate::new(202))
        .mount(server)
        .await;
}

async fn connect(server: &MockServer) -> McpClient {
    let url = format!("{}/mcp", server.uri());
    let transport = Box::new(HttpTransport::new(&url).unwrap());
    let client = McpClient::new(transport, ClientConfig::default());
    client.connect().await.expect("connect");
    client
}

async fn get_requests(server: &MockServer) -> usize {
    server
        .received_requests()
        .await
        .unwrap_or_default()
        .iter()
        .filter(|r| r.method == wiremock::http::Method::GET)
        .count()
}

#[tokio::test]
async fn a_2026_connection_never_issues_the_removed_get_sse_stream() {
    let server = MockServer::start().await;
    mount_sse_405(&server).await;
    mount_discover_2026(&server).await;

    let client = connect(&server).await;
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2026_07_28)
    );

    // The listener spawns detached; give it a window to make the mistake.
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;

    assert_eq!(
        get_requests(&server).await,
        0,
        "2026-07-28 removed the GET SSE stream — the client must not open one"
    );
}

#[tokio::test]
async fn a_2025_connection_still_opens_the_get_sse_stream_with_its_session_id() {
    let server = MockServer::start().await;
    mount_sse_405(&server).await;
    mount_2025_handshake(&server).await;

    let client = connect(&server).await;
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2025_11_25)
    );

    let mut sent_with_session = false;
    for _ in 0..40 {
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        let requests = server.received_requests().await.unwrap_or_default();
        if requests.iter().any(|r| {
            r.method == wiremock::http::Method::GET
                && r.headers
                    .get("mcp-session-id")
                    .and_then(|v| v.to_str().ok())
                    == Some("sess-listener")
        }) {
            sent_with_session = true;
            break;
        }
    }
    assert!(
        sent_with_session,
        "the 2025-11-25 GET SSE stream must still open, carrying the negotiated session id"
    );
}
