//! Wire-level acceptance for Origin validation (DNS-rebinding protection).
//!
//! Streamable HTTP §Security: "Servers MUST validate the `Origin` header on
//! all incoming connections to prevent DNS rebinding attacks. If the
//! `Origin` header is present and invalid, servers MUST respond with
//! HTTP 403 Forbidden."
//!
//! Policy contract is ADR-031: default `SameOriginOrLoopback`, additive
//! `AllowList`, opt-out `Disabled`. Origin-absent requests always pass.
//!
//! Built only under the 2026 feature; compiles to nothing under 2025-11-25.
#![cfg(feature = "protocol-2026-07-28")]

mod common;

use turul_http_mcp_server::OriginPolicy;
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;

#[derive(McpTool, Clone, Default)]
#[tool(name = "echo", description = "Echo", output = String)]
struct EchoTool {}

impl EchoTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok("ok".to_string())
    }
}

async fn start_server(policy: Option<OriginPolicy>) -> String {
    let reserved = common::reserve_port().await;
    let port = reserved.port;

    let mut builder = McpServer::builder()
        .name("origin-2026-test")
        .version("0.4.0")
        .tool(EchoTool::default())
        .bind_address(format!("127.0.0.1:{port}").parse().unwrap());
    if let Some(policy) = policy {
        builder = builder.origin_policy(policy);
    }
    let server = builder.build().expect("build 2026 server");

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

fn discover_body() -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "server/discover",
        "params": { "_meta": {
            "io.modelcontextprotocol/protocolVersion": "2026-07-28",
            "io.modelcontextprotocol/clientInfo": { "name": "test-client", "version": "1.0.0" },
            "io.modelcontextprotocol/clientCapabilities": {}
        }}
    })
}

async fn discover_with_origin(url: &str, origin: Option<&str>) -> reqwest::Response {
    let client = reqwest::Client::new();
    let mut req = client
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "server/discover");
    if let Some(o) = origin {
        req = req.header("Origin", o);
    }
    req.json(&discover_body()).send().await.expect("POST")
}

#[tokio::test]
async fn origin_absent_is_allowed() {
    let url = start_server(None).await;
    let resp = discover_with_origin(&url, None).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body["result"]["_meta"]["io.modelcontextprotocol/serverInfo"]["name"].is_string(),
        "{body}"
    );
}

#[tokio::test]
async fn loopback_origin_is_allowed_by_default() {
    let url = start_server(None).await;
    for origin in [
        "http://localhost:3000",
        "http://127.0.0.1:9999",
        "http://[::1]:8080",
    ] {
        let resp = discover_with_origin(&url, Some(origin)).await;
        assert_eq!(resp.status(), 200, "loopback origin {origin} must pass");
    }
}

#[tokio::test]
async fn same_host_origin_is_allowed_by_default() {
    let url = start_server(None).await;
    // Origin matching the request Host authority exactly.
    let host = url
        .strip_prefix("http://")
        .unwrap()
        .strip_suffix("/mcp")
        .unwrap();
    let resp = discover_with_origin(&url, Some(&format!("http://{host}"))).await;
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn cross_origin_is_rejected_with_403_by_default() {
    let url = start_server(None).await;
    for origin in [
        "http://attacker.example",
        "https://evil.example:8443",
        "null",
    ] {
        let resp = discover_with_origin(&url, Some(origin)).await;
        assert_eq!(
            resp.status(),
            403,
            "invalid origin {origin} MUST get 403 Forbidden"
        );
    }
}

#[tokio::test]
async fn allowlist_admits_named_origin_and_keeps_default_semantics() {
    let url = start_server(Some(OriginPolicy::AllowList(vec![
        "https://app.example".to_string(),
    ])))
    .await;

    // Allowlisted origin passes (default-port normalization: 443 implied).
    let resp = discover_with_origin(&url, Some("https://app.example")).await;
    assert_eq!(resp.status(), 200);
    let resp = discover_with_origin(&url, Some("https://app.example:443")).await;
    assert_eq!(
        resp.status(),
        200,
        "default-port normalized form must match"
    );

    // Loopback still passes (allowlist is additive to the default rules).
    let resp = discover_with_origin(&url, Some("http://localhost:3000")).await;
    assert_eq!(resp.status(), 200);

    // Unlisted origin still rejected.
    let resp = discover_with_origin(&url, Some("https://other.example")).await;
    assert_eq!(resp.status(), 403);
}

#[tokio::test]
async fn disabled_policy_skips_validation() {
    let url = start_server(Some(OriginPolicy::Disabled)).await;
    let resp = discover_with_origin(&url, Some("http://attacker.example")).await;
    assert_eq!(resp.status(), 200);
}

/// OPTIONS preflight is exempt (ADR-031): the actual request is what gets
/// gated, and the gated POST after a passing preflight still returns 403.
#[tokio::test]
async fn options_preflight_is_exempt_but_actual_request_is_gated() {
    let url = start_server(None).await;
    let client = reqwest::Client::new();
    let resp = client
        .request(reqwest::Method::OPTIONS, &url)
        .header("Origin", "http://attacker.example")
        .header("Access-Control-Request-Method", "POST")
        .send()
        .await
        .expect("OPTIONS");
    assert_eq!(resp.status(), 200, "preflight passes");

    let resp = discover_with_origin(&url, Some("http://attacker.example")).await;
    assert_eq!(resp.status(), 403, "actual request is rejected");
}
