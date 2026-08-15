//! Wire-level acceptance: the 2026-07-28 `ClientNotification` union dropped
//! `ProgressNotification`, and `notifications/message` was never a member on
//! any pin. On this lane the server's dispatch table has no entry for either
//! method — inbound POSTs still get HTTP 202 Accepted (Streamable HTTP §Server
//! Validation: notifications are acknowledged regardless of dispatch match),
//! but no handler runs. See `McpServerBuilder::build`'s dispatch-table unit
//! tests (`builder.rs`) for the handler-absence proof; this file proves the
//! wire contract holds around that absence.
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

async fn start_server() -> String {
    let reserved = common::reserve_port().await;
    let port = reserved.port;

    let server = McpServer::builder()
        .name("notifications-2026-test")
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

async fn post_notification(
    url: &str,
    method: &str,
    params: serde_json::Value,
) -> reqwest::Response {
    let client = reqwest::Client::new();
    client
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", method)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }))
        .send()
        .await
        .unwrap_or_else(|e| panic!("POST {method} failed: {e}"))
}

/// `notifications/progress` is no longer a `ClientNotification` member on
/// 2026-07-28 — the dispatch table has no entry for it, but the POST
/// still gets 202 per the notification-acknowledgment contract.
#[tokio::test]
async fn inbound_progress_notification_still_gets_202_with_no_dispatch_entry() {
    let url = start_server().await;
    let resp = post_notification(
        &url,
        "notifications/progress",
        serde_json::json!({ "progressToken": "tok-1", "progress": 0.5 }),
    )
    .await;
    assert_eq!(
        resp.status(),
        202,
        "inbound notifications/progress must still be acknowledged with 202 \
         even though the 2026 lane has no handler registered for it"
    );
}

/// `notifications/message` was never a `ClientNotification` member on any
/// pin — same contract: 202 with no dispatch entry.
#[tokio::test]
async fn inbound_message_notification_still_gets_202_with_no_dispatch_entry() {
    let url = start_server().await;
    let resp = post_notification(
        &url,
        "notifications/message",
        serde_json::json!({ "level": "info", "data": "hello" }),
    )
    .await;
    assert_eq!(
        resp.status(),
        202,
        "inbound notifications/message must still be acknowledged with 202 \
         even though the 2026 lane has no handler registered for it"
    );
}
