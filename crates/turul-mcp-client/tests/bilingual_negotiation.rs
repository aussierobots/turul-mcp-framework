//! Acceptance tests for ADR-030 per-connection version negotiation.
//!
//! One bilingual `McpClient` (the default build) connects to three different
//! wiremock servers and locks the correct wire spec for each: 2026-07-28 when
//! the server answers `server/discover`, 2025-11-25 when it returns `-32601`
//! (no discover), and aborts WITHOUT downgrade when the probe gets an HTTP 4xx.
//! This is the executable form of the "one client speaks both specs" minimum.

use turul_mcp_client::config::ClientConfig;
use turul_mcp_client::transport::http::HttpTransport;
use turul_mcp_client::{McpClient, McpVersion};
use wiremock::matchers::{body_partial_json, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// The SSE GET listener fires on connect; return 404 so it exits cleanly
/// (v0.3.38 4xx-terminal contract) without disrupting the test.
async fn mount_sse_404(server: &MockServer) {
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .mount(server)
        .await;
}

async fn connect_client(server: &MockServer) -> (McpClient, turul_mcp_client::McpClientResult<()>) {
    let url = format!("{}/mcp", server.uri());
    let transport = Box::new(HttpTransport::new(&url).unwrap());
    let client = McpClient::new(transport, ClientConfig::default());
    let result = client.connect().await;
    (client, result)
}

#[tokio::test]
async fn bilingual_client_locks_2026_when_server_answers_discover() {
    let server = MockServer::start().await;
    mount_sse_404(&server).await;

    // server/discover returns a valid DiscoverResult => this is a 2026 server.
    Mock::given(method("POST"))
        .and(body_partial_json(serde_json::json!({"method": "server/discover"})))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": "req_0",
                    "result": {
                        "resultType": "complete",
                        "ttlMs": 0,
                        "cacheScope": "public",
                        "supportedVersions": ["2026-07-28"],
                        "capabilities": {},
                        "serverInfo": { "name": "mock-2026", "version": "1.0.0" }
                    }
                })),
        )
        .mount(&server)
        .await;

    let (client, result) = connect_client(&server).await;
    result.expect("connect against a 2026 server must succeed");
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2026_07_28),
        "a server answering server/discover must lock the connection to 2026-07-28"
    );
}

#[tokio::test]
async fn bilingual_client_locks_2025_when_server_lacks_discover() {
    let server = MockServer::start().await;
    mount_sse_404(&server).await;

    // A real 2025-11-25 server has no server/discover => -32601 => fall back.
    Mock::given(method("POST"))
        .and(body_partial_json(serde_json::json!({"method": "server/discover"})))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": "req_0",
                    "error": { "code": -32601, "message": "Method not found" }
                })),
        )
        .mount(&server)
        .await;

    // initialize: session ID + minimal valid InitializeResult.
    Mock::given(method("POST"))
        .and(body_partial_json(serde_json::json!({"method": "initialize"})))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .insert_header("Mcp-Session-Id", "sess-bilingual")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": "req_0",
                    "result": {
                        "protocolVersion": "2025-11-25",
                        "capabilities": { "tools": { "listChanged": false } },
                        "serverInfo": { "name": "mock-2025", "version": "1.0.0" }
                    }
                })),
        )
        .mount(&server)
        .await;

    // notifications/initialized: 202 Accepted per MCP spec.
    Mock::given(method("POST"))
        .and(body_partial_json(serde_json::json!({"method": "notifications/initialized"})))
        .respond_with(ResponseTemplate::new(202))
        .mount(&server)
        .await;

    let (client, result) = connect_client(&server).await;
    result.expect("connect against a 2025 server must succeed via fallback");
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2025_11_25),
        "a -32601 on server/discover must fall back to and lock 2025-11-25"
    );
}

#[tokio::test]
async fn bilingual_client_aborts_on_4xx_without_downgrade() {
    let server = MockServer::start().await;
    mount_sse_404(&server).await;

    // HTTP 403 on the probe is an authorization failure, NOT a version signal.
    Mock::given(method("POST"))
        .and(body_partial_json(serde_json::json!({"method": "server/discover"})))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;

    let (client, result) = connect_client(&server).await;
    assert!(
        result.is_err(),
        "HTTP 403 on server/discover must abort the connect, not silently downgrade"
    );
    assert_eq!(
        client.negotiated_version().await,
        None,
        "no wire version may be locked when negotiation aborts"
    );
}
