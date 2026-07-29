//! Wire-level acceptance for the 2026-07-28 HTTP surface of the MCP endpoint.
//!
//! The stateless core accepts POST only. Per the Streamable HTTP binding's
//! Backward Compatibility rules, a server that supports only this revision
//! answers legacy-era traffic as follows:
//!   - HTTP GET or DELETE to the MCP endpoint → `405 Method Not Allowed`
//!   - An `Mcp-Session-Id` header on a request → ignored; the server neither
//!     honors it nor mints/echoes session ids
//!   - A `Last-Event-ID` header → ignored; streams are not resumable
//!
//! Built only under the 2026 feature; compiles to nothing under 2025-11-25.
#![cfg(feature = "protocol-2026-07-28")]

mod common;

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
    let reserved = common::reserve_port().await;
    let port = reserved.port;

    let server = McpServer::builder()
        .name("surface-2026-test")
        .version("0.4.0")
        .tool(EchoTool::default())
        .bind_address(format!("127.0.0.1:{port}").parse().unwrap())
        .build()
        .expect("build 2026 server");

    tokio::spawn(async move {
        server.run().await.ok();
    });

    let url = format!("http://127.0.0.1:{port}/mcp");
    let client = reqwest::Client::new();
    for _ in 0..50 {
        if client.get(&url).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    url
}

/// A spec-complete per-request `RequestMetaObject`.
fn meta() -> serde_json::Value {
    serde_json::json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "test-client", "version": "1.0.0" },
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

#[tokio::test]
async fn get_returns_405_method_not_allowed() {
    let url = start_server().await;
    let client = reqwest::Client::new();

    // Bare GET (no MCP headers at all).
    let resp = client.get(&url).send().await.expect("bare GET");
    assert_eq!(
        resp.status(),
        405,
        "GET to the 2026 MCP endpoint must return 405 Method Not Allowed"
    );
    let allow = resp
        .headers()
        .get("Allow")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        allow.contains("POST"),
        "405 response must carry an Allow header listing POST, got '{allow}'"
    );

    // Legacy-shaped GET: SSE Accept + session id, as a 2025-era client would send.
    let resp = client
        .get(&url)
        .header("Accept", "text/event-stream")
        .header("Mcp-Session-Id", "0123456789abcdef0123456789abcdef")
        .header("MCP-Protocol-Version", "2026-07-28")
        .send()
        .await
        .expect("legacy-shaped GET");
    assert_eq!(
        resp.status(),
        405,
        "legacy GET-SSE traffic must get 405, not an SSE stream or a 400"
    );
}

#[tokio::test]
async fn get_with_last_event_id_returns_405() {
    let url = start_server().await;
    let client = reqwest::Client::new();

    // Resumable SSE via Last-Event-ID is not supported in this revision.
    let resp = client
        .get(&url)
        .header("Accept", "text/event-stream")
        .header("Mcp-Session-Id", "0123456789abcdef0123456789abcdef")
        .header("Last-Event-ID", "42")
        .send()
        .await
        .expect("GET with Last-Event-ID");
    assert_eq!(
        resp.status(),
        405,
        "Last-Event-ID resumption attempts must get 405 — streams are not resumable"
    );
}

#[tokio::test]
async fn delete_returns_405_method_not_allowed() {
    let url = start_server().await;
    let client = reqwest::Client::new();

    // Legacy session termination, as a 2025-era client would send.
    let resp = client
        .delete(&url)
        .header("Accept", "application/json")
        .header("Mcp-Session-Id", "0123456789abcdef0123456789abcdef")
        .send()
        .await
        .expect("legacy DELETE");
    assert_eq!(
        resp.status(),
        405,
        "DELETE to the 2026 MCP endpoint must return 405 Method Not Allowed"
    );
    let allow = resp
        .headers()
        .get("Allow")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        allow.contains("POST"),
        "405 response must carry an Allow header listing POST, got '{allow}'"
    );
}

#[tokio::test]
async fn inbound_mcp_session_id_is_ignored_and_never_echoed() {
    let url = start_server().await;
    let client = reqwest::Client::new();

    // A request carrying a (bogus) legacy session header must succeed exactly
    // as if the header were absent — not honored, not validated, not 404'd.
    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "echo")
        .header("Mcp-Session-Id", "ffffffffffffffffffffffffffffffff")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": { "message": "hi" },
                "_meta": meta()
            }
        }))
        .send()
        .await
        .expect("POST with legacy session header");

    assert_eq!(
        resp.status(),
        200,
        "the session header must be ignored, not treated as a stale session"
    );
    assert!(
        resp.headers().get("Mcp-Session-Id").is_none(),
        "a 2026 server must not echo Mcp-Session-Id response headers"
    );
    let body: serde_json::Value = resp.json().await.expect("json body");
    assert!(
        body["result"].is_object(),
        "tools/call must dispatch normally, got: {body}"
    );
}

#[tokio::test]
async fn responses_never_mint_session_ids() {
    let url = start_server().await;
    let client = reqwest::Client::new();

    // Plain stateless request: no session header in, none out.
    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/list")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/list",
            "params": { "_meta": meta() }
        }))
        .send()
        .await
        .expect("sessionless tools/list");
    assert_eq!(resp.status(), 200);
    assert!(
        resp.headers().get("Mcp-Session-Id").is_none(),
        "a 2026 server must not mint or echo session ids on request responses"
    );

    // Accepted notifications return 202 with no session header either.
    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "notifications/progress")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "method": "notifications/progress",
            "params": {
                "progressToken": "t1",
                "progress": 0.5
            }
        }))
        .send()
        .await
        .expect("notification POST");
    assert_eq!(
        resp.status(),
        202,
        "accepted notifications must return 202 Accepted"
    );
    assert!(
        resp.headers().get("Mcp-Session-Id").is_none(),
        "a 2026 server must not mint or echo session ids on notification acks"
    );
}
