//! Wire-level acceptance for the DRAFT-2026-v1 stateless server core.
//!
//! Proves, against a real HTTP server built for the 2026 spec, that:
//!   1. `server/discover` answers without any session and returns a wire-shaped
//!      `DiscoverResult` (`resultType: "complete"`, `supportedVersions`,
//!      `capabilities`, `serverInfo`).
//!   2. `tools/call` dispatches with NO `Mcp-Session-Id` and NO prior
//!      `initialize`/`initialized` handshake — the stateless core never answers
//!      a sessionless request with HTTP 400.
//!
//! Built only under the 2026 feature; compiles to nothing under 2025-11-25.
#![cfg(feature = "protocol-2026-07-28")]

use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;

#[derive(McpTool, Clone, Default)]
#[tool(name = "echo", description = "Echo back the provided message", output = String)]
struct EchoTool {
    #[param(description = "Message to echo back")]
    message: String,
}

impl EchoTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok(format!("Echo: {}", self.message))
    }
}

/// Start a 2026 server on an ephemeral port and return its `/mcp` URL once it
/// accepts connections.
async fn start_server() -> String {
    // Reserve a free port, then hand it to the server. The brief gap between
    // dropping this listener and the server binding is the standard test pattern.
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();

    let server = McpServer::builder()
        .name("discover-2026-test")
        .version("0.4.0")
        .tool(EchoTool::default())
        .with_resources()
        .with_prompts()
        .bind_address(format!("127.0.0.1:{port}").parse().unwrap())
        .build()
        .expect("build 2026 server");

    tokio::spawn(async move {
        server.run().await.ok();
    });

    let url = format!("http://127.0.0.1:{port}/mcp");
    // Wait until the accept loop is live (build() binds; run() starts accepting).
    let client = reqwest::Client::new();
    for _ in 0..50 {
        if client.get(&url).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    url
}

/// A spec-complete per-request `RequestMetaObject` — the 2026 core requires
/// `protocolVersion`, `clientInfo`, and `clientCapabilities` on every request.
fn meta() -> serde_json::Value {
    serde_json::json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "test-client", "version": "1.0.0" },
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

#[tokio::test]
async fn server_discover_answers_without_a_session() {
    let url = start_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "server/discover",
            "params": { "_meta": meta() }
        }))
        .send()
        .await
        .expect("server/discover POST");

    assert_eq!(
        resp.status(),
        200,
        "server/discover must succeed without an Mcp-Session-Id"
    );
    // The server must advertise 2026-07-28 on the wire, not fall back to 2025-11-25.
    assert_eq!(
        resp.headers()
            .get("MCP-Protocol-Version")
            .and_then(|v| v.to_str().ok()),
        Some("2026-07-28"),
        "a 2026 server must echo MCP-Protocol-Version: 2026-07-28"
    );
    let body: serde_json::Value = resp.json().await.expect("json body");
    assert_eq!(body["result"]["resultType"], "complete");
    assert_eq!(
        body["result"]["supportedVersions"][0], "2026-07-28",
        "server must advertise the 2026 protocol version"
    );
    assert!(body["result"]["capabilities"].is_object());
    assert_eq!(body["result"]["serverInfo"]["name"], "discover-2026-test");
}

#[tokio::test]
async fn tools_call_dispatches_without_session_handshake() {
    let url = start_server().await;
    let client = reqwest::Client::new();

    // No initialize, no notifications/initialized, no Mcp-Session-Id header.
    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": {
                "_meta": meta(),
                "name": "echo",
                "arguments": { "message": "hi" }
            }
        }))
        .send()
        .await
        .expect("tools/call POST");

    assert_eq!(
        resp.status(),
        200,
        "stateless tools/call must dispatch without a session (never HTTP 400)"
    );
    let body: serde_json::Value = resp.json().await.expect("json body");
    assert!(
        body.get("error").is_none(),
        "tools/call must not error on a sessionless 2026 request: {body}"
    );
    let text = body["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default();
    assert!(
        text.contains("Echo: hi"),
        "unexpected tool result shape: {body}"
    );
}

/// Sends a sessionless list request and returns the parsed JSON-RPC body.
async fn list_request(url: &str, rpc_method: &str) -> serde_json::Value {
    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 9, "method": rpc_method,
            "params": { "_meta": meta() }
        }))
        .send()
        .await
        .unwrap_or_else(|_| panic!("{rpc_method} POST"));
    assert_eq!(
        resp.status(),
        200,
        "stateless {rpc_method} must dispatch without a session"
    );
    resp.json().await.expect("json body")
}

#[tokio::test]
async fn resources_list_dispatches_statelessly_with_cacheable_result() {
    let url = start_server().await;
    let body = list_request(&url, "resources/list").await;
    assert!(body.get("error").is_none(), "resources/list errored: {body}");
    // 2026 list results extend CacheableResult.
    assert_eq!(body["result"]["resultType"], "complete");
    assert!(body["result"]["cacheScope"].is_string(), "missing cacheScope: {body}");
    assert!(body["result"]["resources"].is_array(), "missing resources array: {body}");
}

#[tokio::test]
async fn prompts_list_dispatches_statelessly_with_cacheable_result() {
    let url = start_server().await;
    let body = list_request(&url, "prompts/list").await;
    assert!(body.get("error").is_none(), "prompts/list errored: {body}");
    assert_eq!(body["result"]["resultType"], "complete");
    assert!(body["result"]["cacheScope"].is_string(), "missing cacheScope: {body}");
    assert!(body["result"]["prompts"].is_array(), "missing prompts array: {body}");
}
