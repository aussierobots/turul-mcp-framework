//! Wire-level acceptance for the 2026-07-28 unknown-method mapping.
//!
//! Streamable HTTP §Protocol Version Header: "If the server does not implement
//! the requested RPC method, it MUST respond with `404 Not Found` and a
//! JSON-RPC error with code `-32601` (Method not found). The JSON-RPC error
//! body distinguishes this case from a 404 returned by a legacy HTTP+SSE
//! server that does not host the modern MCP endpoint."
//!
//! Methods absent from the 2026-07-28 schema (`ping`, `initialize`, `tasks/*`,
//! `logging/setLevel`, `resources/subscribe`) are unknown methods here.
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

async fn start_server() -> String {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();

    let server = McpServer::builder()
        .name("errmap-2026-test")
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

fn meta() -> serde_json::Value {
    serde_json::json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "test-client", "version": "1.0.0" },
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

/// POST a fully-headed request for `rpc_method` and return (status, body).
async fn post_method(url: &str, rpc_method: &str) -> (reqwest::StatusCode, serde_json::Value) {
    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", rpc_method)
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": rpc_method,
            "params": { "_meta": meta() }
        }))
        .send()
        .await
        .unwrap_or_else(|e| panic!("{rpc_method} POST failed: {e}"));
    let status = resp.status();
    let body: serde_json::Value = resp.json().await.unwrap_or_default();
    (status, body)
}

#[tokio::test]
async fn unknown_method_gets_http_404_with_method_not_found() {
    let url = start_server().await;
    let (status, body) = post_method(&url, "frobnicate/run").await;
    assert_eq!(status, 404, "unknown method must be HTTP 404, got: {body}");
    assert_eq!(
        body["error"]["code"], -32601,
        "404 body must carry JSON-RPC -32601 so clients can distinguish it \
         from a legacy HTTP+SSE server's 404: {body}"
    );
}

#[tokio::test]
async fn methods_absent_from_the_2026_schema_get_404() {
    let url = start_server().await;
    // ping/initialize/tasks/logging-setLevel/resources-subscribe have no
    // bindings in the pinned 2026-07-28 schema — a 2026-only server does not
    // implement them.
    for method in [
        "ping",
        "initialize",
        "tasks/get",
        "tasks/list",
        "logging/setLevel",
        "resources/subscribe",
    ] {
        let (status, body) = post_method(&url, method).await;
        assert_eq!(
            status, 404,
            "{method} is not a 2026-07-28 method — must be HTTP 404, got: {body}"
        );
        assert_eq!(
            body["error"]["code"], -32601,
            "{method}: 404 body must carry -32601: {body}"
        );
    }
}

#[tokio::test]
async fn known_methods_are_unaffected() {
    let url = start_server().await;
    let (status, body) = post_method(&url, "tools/list").await;
    assert_eq!(status, 200);
    assert!(body["result"].is_object(), "tools/list result: {body}");

    let (status, body) = post_method(&url, "server/discover").await;
    assert_eq!(status, 200);
    assert!(body["result"].is_object(), "server/discover result: {body}");
}
