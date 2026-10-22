//! A buffered Lambda response cannot push, so `subscriptions/listen` must not
//! hold a stream open until the function times out. It is refused at once, the
//! same way any unimplemented method is (HTTP 404 + `-32601`).
#![cfg(feature = "protocol-2026-07-28")]

use std::time::Duration;

use serde_json::json;
use turul_mcp_aws_lambda::LambdaMcpServerBuilder;

fn listen_request() -> lambda_http::Request {
    let body = json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "subscriptions/listen",
        "params": {
            "notifications": { "toolsListChanged": true },
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientInfo": { "name": "test-client", "version": "1.0.0" },
                "io.modelcontextprotocol/clientCapabilities": {}
            }
        }
    });
    http::Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "subscriptions/listen")
        .body(lambda_http::Body::from(
            serde_json::to_string(&body).unwrap(),
        ))
        .unwrap()
}

#[tokio::test]
async fn listen_is_refused_at_once_over_lambda() {
    let server = LambdaMcpServerBuilder::new()
        .name("listen-refused")
        .version("1.0.0")
        .build()
        .await
        .expect("build lambda server");
    let handler = server.handler().await.expect("create lambda handler");

    let response = tokio::time::timeout(Duration::from_secs(2), handler.handle(listen_request()))
        .await
        .expect("subscriptions/listen must return at once, not hold the response open")
        .expect("handle");

    assert_eq!(response.status(), 404);
    let bytes = match response.body() {
        lambda_http::Body::Text(s) => s.as_bytes().to_vec(),
        lambda_http::Body::Binary(b) => b.clone(),
        _ => Vec::new(),
    };
    let body: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON error body");
    assert_eq!(body["error"]["code"], -32601, "{body}");
    assert_eq!(body["id"], 7);
}
