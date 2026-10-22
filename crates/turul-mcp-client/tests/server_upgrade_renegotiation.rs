#![cfg(feature = "client-bilingual")]
//! A connected bilingual `McpClient` whose server is replaced by one speaking
//! the other spec revision must keep working without the application
//! reconnecting by hand.
//!
//! The wire version is locked per connection. When the server behind the URL
//! changes era, the next request is refused with `-32022`
//! (UnsupportedProtocolVersionError). The client re-runs negotiation once. If
//! the connection lands on the same spec the request is re-sent; if the spec
//! changed, the request (built for the old spec) surfaces the `-32022` and the
//! next call runs on the repaired connection. Any other failure is surfaced
//! untouched.

use serde_json::{Value, json};
use turul_mcp_client::config::ClientConfig;
use turul_mcp_client::transport::http::HttpTransport;
use turul_mcp_client::{McpClient, McpVersion};
use wiremock::matchers::{body_partial_json, header, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

const V2025: &str = "2025-11-25";
const V2026: &str = "2026-07-28";

fn json_response(status: u16, body: Value) -> ResponseTemplate {
    ResponseTemplate::new(status)
        .insert_header("Content-Type", "application/json")
        .set_body_json(body)
}

fn rpc_method(name: &str) -> wiremock::matchers::BodyPartialJsonMatcher {
    body_partial_json(json!({ "method": name }))
}

async fn mount_sse_404(server: &MockServer) {
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .mount(server)
        .await;
}

fn tool_result(text: &str) -> Value {
    json!({ "content": [{ "type": "text", "text": text }], "isError": false })
}

fn tool_result_2026(text: &str) -> Value {
    json!({
        "resultType": "complete", "ttlMs": 0, "cacheScope": "public",
        "content": [{ "type": "text", "text": text }], "isError": false
    })
}

fn unsupported_version(supported: &[&str]) -> Value {
    json!({
        "jsonrpc": "2.0", "id": "req_0",
        "error": {
            "code": -32022,
            "message": "Unsupported protocol version",
            "data": { "supported": supported }
        }
    })
}

/// A stateful 2025-11-25 server: no `server/discover`, a normal handshake.
async fn mount_2025_server(server: &MockServer, tool_text: &str) {
    mount_sse_404(server).await;
    Mock::given(method("POST"))
        .and(rpc_method("server/discover"))
        .respond_with(json_response(
            200,
            json!({ "jsonrpc": "2.0", "id": "req_0",
                    "error": { "code": -32601, "message": "Method not found" } }),
        ))
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(rpc_method("initialize"))
        .respond_with(
            json_response(
                200,
                json!({ "jsonrpc": "2.0", "id": "req_0", "result": {
                    "protocolVersion": V2025,
                    "capabilities": { "tools": { "listChanged": false } },
                    "serverInfo": { "name": "mock-2025", "version": "1.0.0" } } }),
            )
            .insert_header("Mcp-Session-Id", "sess-upgrade"),
        )
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(rpc_method("notifications/initialized"))
        .respond_with(ResponseTemplate::new(202))
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(rpc_method("tools/call"))
        .and(header("MCP-Protocol-Version", V2025))
        .respond_with(json_response(
            200,
            json!({ "jsonrpc": "2.0", "id": "req_1", "result": tool_result(tool_text) }),
        ))
        .mount(server)
        .await;
}

/// A stateless 2026-07-28 server: answers `server/discover`, validates the
/// version header, and refuses 2025-versioned requests with `-32022`.
async fn mount_2026_server(server: &MockServer, tool_text: &str) {
    mount_sse_404(server).await;
    Mock::given(method("POST"))
        .and(rpc_method("server/discover"))
        .respond_with(json_response(
            200,
            json!({ "jsonrpc": "2.0", "id": "req_0", "result": {
                "resultType": "complete", "ttlMs": 0, "cacheScope": "public",
                "supportedVersions": [V2026], "capabilities": {},
                "_meta": { "io.modelcontextprotocol/serverInfo":
                           { "name": "mock-2026", "version": "1.0.0" } } } }),
        ))
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(header("MCP-Protocol-Version", V2025))
        .respond_with(json_response(400, unsupported_version(&[V2026])))
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(rpc_method("tools/call"))
        .and(header("MCP-Protocol-Version", V2026))
        .respond_with(json_response(
            200,
            json!({ "jsonrpc": "2.0", "id": "req_1", "result": tool_result_2026(tool_text) }),
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
    client.connect().await.expect("initial connect");
    client
}

async fn call(client: &McpClient) -> turul_mcp_client::McpClientResult<String> {
    client
        .call_tool("echo", json!({ "msg": "hi" }))
        .await
        .map(|r| serde_json::to_string(&r).unwrap_or_default())
}

async fn count(server: &MockServer, rpc: &str) -> usize {
    server
        .received_requests()
        .await
        .unwrap_or_default()
        .iter()
        .filter(|r| {
            r.method.as_str() == "POST"
                && serde_json::from_slice::<Value>(&r.body)
                    .map(|b| b["method"] == rpc)
                    .unwrap_or(false)
        })
        .count()
}

/// The server behind the URL is upgraded from 2025-11-25 to 2026-07-28 while
/// the client stays connected.
#[tokio::test]
async fn live_client_survives_server_upgrade_2025_to_2026() {
    let server = MockServer::start().await;
    mount_2025_server(&server, "from-2025").await;
    let client = connect(&server).await;
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2025_11_25)
    );
    assert!(call(&client).await.unwrap().contains("from-2025"));

    server.reset().await;
    mount_2026_server(&server, "from-2026").await;

    // The request in flight was built for the old spec, so it cannot be
    // re-sent as-is: it fails once while the client re-negotiates...
    let first = call(&client)
        .await
        .expect_err("the in-flight request is refused");
    assert!(first.is_unsupported_protocol_version(), "{first:?}");
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2026_07_28)
    );

    // ...and the connection is already repaired: no manual reconnect.
    let out = call(&client).await.expect("the next call must succeed");
    assert!(out.contains("from-2026"), "{out}");
}

/// The reverse: a rollback from 2026-07-28 to 2025-11-25.
#[tokio::test]
async fn live_client_survives_server_rollback_2026_to_2025() {
    let server = MockServer::start().await;
    mount_2026_server(&server, "from-2026").await;
    let client = connect(&server).await;
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2026_07_28)
    );
    assert!(call(&client).await.unwrap().contains("from-2026"));

    server.reset().await;
    mount_2025_server(&server, "from-2025").await;
    // A real stateful 2025 server does not know the 2026 version header; it
    // refuses the sessionless request for lack of an Mcp-Session-Id.
    Mock::given(method("POST"))
        .and(header("MCP-Protocol-Version", V2026))
        .respond_with(json_response(
            400,
            json!({ "jsonrpc": "2.0", "id": "req_0", "error": {
                "code": -32001,
                "message": "Missing Mcp-Session-Id header. Call initialize first." } }),
        ))
        .with_priority(1)
        .mount(&server)
        .await;

    let first = call(&client)
        .await
        .expect_err("the in-flight request is refused");
    assert!(first.is_missing_session_id(), "{first:?}");
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2025_11_25)
    );

    let out = call(&client).await.expect("the next call must succeed");
    assert!(out.contains("from-2025"), "{out}");
}

/// A server that keeps refusing must not put the client in a loop: one
/// re-negotiation, one retry, then the error surfaces.
#[tokio::test]
async fn renegotiation_is_bounded_to_one_attempt() {
    let server = MockServer::start().await;
    mount_2025_server(&server, "from-2025").await;
    let client = connect(&server).await;

    server.reset().await;
    mount_sse_404(&server).await;
    Mock::given(method("POST"))
        .respond_with(json_response(400, unsupported_version(&[V2025, V2026])))
        .mount(&server)
        .await;

    assert!(
        call(&client).await.is_err(),
        "a permanently refusing server must surface an error"
    );
    assert!(
        count(&server, "server/discover").await <= 1,
        "at most one re-negotiation probe"
    );
    assert!(
        count(&server, "initialize").await <= 1,
        "at most one re-initialize"
    );
}

/// Only the recognised version errors trigger re-negotiation. An unrelated
/// failure mid-session is surfaced as it always was.
#[tokio::test]
async fn unrelated_errors_do_not_trigger_renegotiation() {
    let server = MockServer::start().await;
    mount_2025_server(&server, "from-2025").await;
    let client = connect(&server).await;

    server.reset().await;
    mount_sse_404(&server).await;
    Mock::given(method("POST"))
        .and(rpc_method("tools/call"))
        .respond_with(json_response(
            200,
            json!({ "jsonrpc": "2.0", "id": "req_1",
                    "error": { "code": -32602, "message": "Invalid params" } }),
        ))
        .mount(&server)
        .await;

    assert!(call(&client).await.is_err());
    assert_eq!(
        count(&server, "server/discover").await,
        0,
        "no re-probe on -32602"
    );
    assert_eq!(
        count(&server, "initialize").await,
        0,
        "no re-initialize on -32602"
    );
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2025_11_25)
    );
}

/// `-32001` is also this framework's `Unauthenticated` code. Only the
/// missing-session message is an era signal; an auth failure on a 2026
/// connection is surfaced, not answered with a re-negotiation.
#[tokio::test]
async fn unauthenticated_32001_on_a_2026_connection_does_not_renegotiate() {
    let server = MockServer::start().await;
    mount_2026_server(&server, "from-2026").await;
    let client = connect(&server).await;

    server.reset().await;
    mount_sse_404(&server).await;
    Mock::given(method("POST"))
        .respond_with(json_response(
            200,
            json!({ "jsonrpc": "2.0", "id": "req_1",
                    "error": { "code": -32001, "message": "Unauthenticated" } }),
        ))
        .mount(&server)
        .await;

    assert!(call(&client).await.is_err());
    assert_eq!(count(&server, "server/discover").await, 0);
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2026_07_28)
    );
}

/// Many callers hit the flip together. Each in-flight request may fail, but
/// the client must end up on the new spec and serve the next calls.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_callers_across_the_flip_all_recover() {
    let server = MockServer::start().await;
    mount_2025_server(&server, "from-2025").await;
    let client = std::sync::Arc::new(connect(&server).await);

    server.reset().await;
    mount_2026_server(&server, "from-2026").await;

    let burst: Vec<_> = (0..8)
        .map(|_| {
            let c = client.clone();
            tokio::spawn(async move { call(&c).await })
        })
        .collect();
    for h in burst {
        let _ = h.await.unwrap();
    }

    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2026_07_28)
    );
    for _ in 0..4 {
        let out = call(&client).await.expect("calls after the burst succeed");
        assert!(out.contains("from-2026"), "{out}");
    }
}
