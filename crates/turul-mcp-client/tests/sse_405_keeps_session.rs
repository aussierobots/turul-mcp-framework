#![cfg(feature = "client-bilingual")]
//! A 2025-11-25 server that offers no GET stream answers the listener's GET
//! with 405 ("the server does not offer an SSE stream at this endpoint"). That
//! is not session loss: the session id established by `initialize` must stay,
//! and later requests must still carry it.

use std::time::Duration;

use serde_json::json;
use turul_mcp_client::config::ClientConfig;
use turul_mcp_client::transport::http::HttpTransport;
use turul_mcp_client::{McpClient, McpVersion};
use wiremock::matchers::{body_partial_json, header, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn rpc(name: &str) -> wiremock::matchers::BodyPartialJsonMatcher {
    body_partial_json(json!({ "method": name }))
}

fn json_body(status: u16, body: serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(status)
        .insert_header("Content-Type", "application/json")
        .set_body_json(body)
}

async fn mount_2025_server(server: &MockServer, get_status: u16) {
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(get_status))
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(rpc("server/discover"))
        .respond_with(json_body(
            200,
            json!({ "jsonrpc": "2.0", "id": "req_0",
                    "error": { "code": -32601, "message": "Method not found" } }),
        ))
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(rpc("initialize"))
        .respond_with(
            json_body(
                200,
                json!({ "jsonrpc": "2.0", "id": "req_0", "result": {
                    "protocolVersion": "2025-11-25",
                    "capabilities": { "tools": { "listChanged": false } },
                    "serverInfo": { "name": "mock-2025", "version": "1.0.0" } } }),
            )
            .insert_header("Mcp-Session-Id", "sess-405"),
        )
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(rpc("notifications/initialized"))
        .respond_with(ResponseTemplate::new(202))
        .mount(server)
        .await;
    // A stateful server refuses a request that lacks the session id.
    Mock::given(method("POST"))
        .and(rpc("tools/call"))
        .and(header("Mcp-Session-Id", "sess-405"))
        .respond_with(json_body(
            200,
            json!({ "jsonrpc": "2.0", "id": "req_1", "result": {
                "content": [{ "type": "text", "text": "ok" }], "isError": false } }),
        ))
        .with_priority(1)
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(rpc("tools/call"))
        .respond_with(json_body(
            200,
            json!({ "jsonrpc": "2.0", "id": "req_1", "error": {
                "code": -32001,
                "message": "Missing Mcp-Session-Id header. Call initialize first." } }),
        ))
        .mount(server)
        .await;
}

async fn connect(server: &MockServer) -> McpClient {
    let url = format!("{}/mcp", server.uri());
    let client = McpClient::new(
        Box::new(HttpTransport::new(&url).unwrap()),
        ClientConfig::default(),
    );
    client.connect().await.expect("connect");
    client
}

/// Wait until the listener has issued its GET and had time to act on the answer.
async fn let_listener_settle(server: &MockServer) {
    for _ in 0..100 {
        let got_get = server
            .received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .any(|r| r.method.as_str() == "GET");
        if got_get {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
}

#[tokio::test]
async fn get_405_no_stream_keeps_the_session_id() {
    let server = MockServer::start().await;
    mount_2025_server(&server, 405).await;
    let client = connect(&server).await;
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2025_11_25)
    );

    let_listener_settle(&server).await;

    let out = client
        .call_tool("echo", json!({}))
        .await
        .expect("a 405 on the GET listener is not session loss; the call must carry the session");
    assert!(serde_json::to_string(&out).unwrap().contains("ok"));
    assert_eq!(
        client.connection_status().await.session_id.as_deref(),
        Some("sess-405")
    );
}

/// 404 on the GET means the server terminated the session. The spec's recovery
/// is a new InitializeRequest, performed when a request gets 404. The listener
/// must not strip the id first: a sessionless request is refused with -32001
/// (not 404) and nothing would trigger recovery.
#[tokio::test]
async fn get_404_session_terminated_recovers_on_the_next_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(rpc("server/discover"))
        .respond_with(json_body(
            200,
            json!({ "jsonrpc": "2.0", "id": "req_0",
                    "error": { "code": -32601, "message": "Method not found" } }),
        ))
        .mount(&server)
        .await;
    let init = |sid: &'static str| {
        json_body(
            200,
            json!({ "jsonrpc": "2.0", "id": "req_0", "result": {
                "protocolVersion": "2025-11-25",
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "mock-2025", "version": "1.0.0" } } }),
        )
        .insert_header("Mcp-Session-Id", sid)
    };
    // The first handshake mints sess-A; after termination the second mints sess-B.
    Mock::given(method("POST"))
        .and(rpc("initialize"))
        .respond_with(init("sess-A"))
        .up_to_n_times(1)
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(rpc("initialize"))
        .respond_with(init("sess-B"))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(rpc("notifications/initialized"))
        .respond_with(ResponseTemplate::new(202))
        .mount(&server)
        .await;
    // sess-A is terminated: the server answers 404 to any request carrying it.
    Mock::given(method("POST"))
        .and(rpc("tools/call"))
        .and(header("Mcp-Session-Id", "sess-A"))
        .respond_with(ResponseTemplate::new(404))
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(rpc("tools/call"))
        .and(header("Mcp-Session-Id", "sess-B"))
        .respond_with(json_body(
            200,
            json!({ "jsonrpc": "2.0", "id": "req_1", "result": {
                "content": [{ "type": "text", "text": "ok" }], "isError": false } }),
        ))
        .with_priority(2)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(rpc("tools/call"))
        .respond_with(json_body(
            200,
            json!({ "jsonrpc": "2.0", "id": "req_1", "error": {
                "code": -32001,
                "message": "Missing Mcp-Session-Id header. Call initialize first." } }),
        ))
        .mount(&server)
        .await;

    let client = connect(&server).await;
    let_listener_settle(&server).await;

    let out = client
        .call_tool("echo", json!({}))
        .await
        .expect("the 404 recovery must start a new session and succeed");
    assert!(serde_json::to_string(&out).unwrap().contains("ok"));
    assert_eq!(
        client.connection_status().await.session_id.as_deref(),
        Some("sess-B")
    );
}
