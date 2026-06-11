//! Wire-level acceptance for disconnect-as-cancellation on the 2026 path.
//!
//! Streamable HTTP §Cancellation: "Closing the SSE response stream MUST be
//! treated by the server as cancellation of that request. The server SHOULD
//! stop work on the cancelled request as soon as practical and MUST NOT
//! send any further messages for it."
//!
//! Built only under the 2026 feature; compiles to nothing under 2025-11-25.
#![cfg(feature = "protocol-2026-07-28")]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use turul_mcp_builders::ToolBuilder;
use turul_mcp_server::prelude::*;

/// Server with one slow tool that flips `completed` only if it runs to the end.
async fn start_server(completed: Arc<AtomicBool>) -> String {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();

    let slow_tool = ToolBuilder::new("slow")
        .description("Sleeps, then records completion")
        .number_output()
        .execute(move |_args| {
            let completed = completed.clone();
            async move {
                tokio::time::sleep(Duration::from_millis(1200)).await;
                completed.store(true, Ordering::SeqCst);
                Ok(serde_json::json!({ "result": 1.0 }))
            }
        })
        .build()
        .expect("build slow tool");

    let server = McpServer::builder()
        .name("cancel-2026-test")
        .version("0.4.0")
        .tool(slow_tool)
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
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    url
}

fn call_body() -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": { "name": "slow", "arguments": {}, "_meta": {
            "io.modelcontextprotocol/protocolVersion": "2026-07-28",
            "io.modelcontextprotocol/clientInfo": { "name": "test-client", "version": "1.0.0" },
            "io.modelcontextprotocol/clientCapabilities": {}
        }}
    })
}

fn request(client: &reqwest::Client, url: &str) -> reqwest::RequestBuilder {
    client
        .post(url)
        .header("Accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "slow")
        .json(&call_body())
}

/// Control: an undisturbed request runs the tool to completion.
#[tokio::test]
async fn undisturbed_request_completes_the_tool() {
    let completed = Arc::new(AtomicBool::new(false));
    let url = start_server(completed.clone()).await;
    let client = reqwest::Client::new();
    let resp = request(&client, &url).send().await.expect("POST");
    assert_eq!(resp.status(), 200);
    let _body = resp.bytes().await.expect("drain response");
    assert!(
        completed.load(Ordering::SeqCst),
        "control: the tool must complete when the client stays connected"
    );
}

/// Closing the SSE response stream mid-execution MUST cancel the request:
/// the server stops work (the tool never reaches its completion line).
#[tokio::test]
async fn client_disconnect_cancels_the_in_flight_request() {
    let completed = Arc::new(AtomicBool::new(false));
    let url = start_server(completed.clone()).await;

    // Drop client + response while the tool is still sleeping: connection
    // closes, which is the transport's cancellation signal.
    {
        let client = reqwest::Client::new();
        let resp = request(&client, &url).send().await.expect("POST");
        assert_eq!(resp.status(), 200, "stream opens before the tool finishes");
        tokio::time::sleep(Duration::from_millis(200)).await;
        drop(resp);
        drop(client);
    }

    // Give the server ample time: if dispatch were still running detached,
    // the tool would flip the flag at ~1200ms.
    tokio::time::sleep(Duration::from_millis(2300)).await;
    assert!(
        !completed.load(Ordering::SeqCst),
        "the server MUST treat the closed response stream as cancellation \
         and stop work on the request"
    );
}
