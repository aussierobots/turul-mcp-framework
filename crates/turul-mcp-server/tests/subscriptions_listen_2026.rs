//! Wire-level acceptance for `subscriptions/listen` (2026-07-28).
//!
//! Subscriptions pattern contract:
//!   - The response to `subscriptions/listen` is a long-lived SSE stream.
//!   - The server MUST send `notifications/subscriptions/acknowledged` as the
//!     first message on the stream, echoing the honored filter subset.
//!   - The server MUST NOT send notification types the client did not request.
//!   - Every notification on the stream carries
//!     `io.modelcontextprotocol/subscriptionId` in `_meta`, matching the id of
//!     the `subscriptions/listen` request.
//!
//! Built only under the 2026 feature; compiles to nothing under 2025-11-25.
#![cfg(feature = "protocol-2026-07-28")]

use std::collections::HashMap;
use std::time::Duration;

use futures::StreamExt;
use turul_http_mcp_server::notification_bridge::SharedNotificationBroadcaster;
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;
use turul_rpc::JsonRpcNotification;

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

/// Broadcasts one notification of each list-changed flavor plus two
/// `resources/updated` events (one watched URI, one unwatched), exercising the
/// real server-wide broadcast pipeline a production server uses.
#[derive(McpTool, Clone, Default)]
#[tool(name = "emit_changes", description = "Broadcast change notifications", output = String)]
struct EmitChangesTool {}

impl EmitChangesTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        let session = session.ok_or_else(|| McpError::tool_execution("session required"))?;
        let any = session
            .broadcaster
            .as_ref()
            .ok_or_else(|| McpError::tool_execution("broadcaster required"))?;
        let broadcaster = any
            .downcast_ref::<SharedNotificationBroadcaster>()
            .ok_or_else(|| McpError::tool_execution("broadcaster type mismatch"))?
            .clone();

        let _ = broadcaster
            .broadcast_to_all_sessions(JsonRpcNotification::new_no_params(
                "notifications/resources/list_changed".to_string(),
            ))
            .await;
        let _ = broadcaster
            .broadcast_to_all_sessions(JsonRpcNotification::new_no_params(
                "notifications/prompts/list_changed".to_string(),
            ))
            .await;

        let mut watched = HashMap::new();
        watched.insert("uri".to_string(), serde_json::json!("file:///watched.txt"));
        let _ = broadcaster
            .broadcast_to_all_sessions(JsonRpcNotification::new_with_object_params(
                "notifications/resources/updated".to_string(),
                watched,
            ))
            .await;

        let mut unwatched = HashMap::new();
        unwatched.insert("uri".to_string(), serde_json::json!("file:///other.txt"));
        let _ = broadcaster
            .broadcast_to_all_sessions(JsonRpcNotification::new_with_object_params(
                "notifications/resources/updated".to_string(),
                unwatched,
            ))
            .await;

        Ok("emitted".to_string())
    }
}

async fn start_server() -> String {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();

    let server = McpServer::builder()
        .name("subscriptions-2026-test")
        .version("0.4.0")
        .tool(EchoTool::default())
        .tool(EmitChangesTool::default())
        .with_resources()
        .with_prompts()
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

fn meta() -> serde_json::Value {
    serde_json::json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "test-client", "version": "1.0.0" },
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

/// Incrementally parses `data:` payloads out of an SSE byte stream.
struct SseReader<S> {
    stream: S,
    buffer: String,
}

impl<S> SseReader<S>
where
    S: futures::Stream<Item = reqwest::Result<bytes::Bytes>> + Unpin,
{
    fn new(stream: S) -> Self {
        Self {
            stream,
            buffer: String::new(),
        }
    }

    /// Next JSON `data:` payload, or None on timeout/stream end.
    async fn next_json(&mut self, timeout: Duration) -> Option<serde_json::Value> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if let Some(json) = self.pop_event() {
                return Some(json);
            }
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return None;
            }
            match tokio::time::timeout(remaining, self.stream.next()).await {
                Ok(Some(Ok(chunk))) => {
                    self.buffer.push_str(&String::from_utf8_lossy(&chunk));
                }
                _ => return None,
            }
        }
    }

    fn pop_event(&mut self) -> Option<serde_json::Value> {
        while let Some(pos) = self.buffer.find("\n\n") {
            let event: String = self.buffer.drain(..pos + 2).collect();
            for line in event.lines() {
                if let Some(data) = line.strip_prefix("data: ")
                    && let Ok(json) = serde_json::from_str(data)
                {
                    return Some(json);
                }
            }
        }
        None
    }
}

#[tokio::test]
async fn listen_acks_first_then_delivers_only_requested_types() {
    let url = start_server().await;
    let client = reqwest::Client::new();

    // Open the listen stream: resources list changes + one watched URI.
    // promptsListChanged is deliberately NOT requested.
    let resp = client
        .post(&url)
        .header("Accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "subscriptions/listen")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 7, "method": "subscriptions/listen",
            "params": {
                "notifications": {
                    "resourcesListChanged": true,
                    "resourceSubscriptions": ["file:///watched.txt"]
                },
                "_meta": meta()
            }
        }))
        .send()
        .await
        .expect("subscriptions/listen POST");

    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok()),
        Some("text/event-stream"),
        "the listen response must be an SSE stream"
    );

    let mut reader = SseReader::new(resp.bytes_stream());

    // First message MUST be the acknowledgement, carrying the subscription id
    // (the JSON-RPC id of the listen request) and the honored filter subset.
    let ack = reader
        .next_json(Duration::from_secs(5))
        .await
        .expect("acknowledgement frame");
    assert_eq!(
        ack["jsonrpc"], "2.0",
        "ack must be a wire-complete JSON-RPC notification"
    );
    assert_eq!(ack["method"], "notifications/subscriptions/acknowledged");
    assert_eq!(
        ack["params"]["_meta"]["io.modelcontextprotocol/subscriptionId"], "7",
        "subscriptionId must match the listen request id"
    );
    assert_eq!(ack["params"]["notifications"]["resourcesListChanged"], true);
    assert_eq!(
        ack["params"]["notifications"]["resourceSubscriptions"][0],
        "file:///watched.txt"
    );
    assert!(
        ack["params"]["notifications"]
            .as_object()
            .unwrap()
            .get("promptsListChanged")
            .is_none(),
        "types the client did not request must not be acknowledged"
    );

    // Trigger server-wide broadcasts via a normal tools/call on a separate
    // (stateless) request.
    let emit = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "emit_changes")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 8, "method": "tools/call",
            "params": { "name": "emit_changes", "arguments": {}, "_meta": meta() }
        }))
        .send()
        .await
        .expect("emit_changes POST");
    assert_eq!(emit.status(), 200);
    let emit_body: serde_json::Value = emit.json().await.expect("emit body");
    assert!(
        emit_body["result"].is_object(),
        "emit_changes must succeed, got: {emit_body}"
    );

    // Exactly two notifications must arrive: resources/list_changed and the
    // watched resources/updated. prompts/list_changed (not requested) and the
    // unwatched URI must be filtered out.
    let mut delivered = Vec::new();
    while let Some(event) = reader.next_json(Duration::from_secs(3)).await {
        delivered.push(event);
        if delivered.len() >= 2 {
            // Allow a brief grace window for any (incorrect) extra deliveries.
            if let Some(extra) = reader.next_json(Duration::from_millis(500)).await {
                delivered.push(extra);
            }
            break;
        }
    }

    let methods: Vec<&str> = delivered
        .iter()
        .filter_map(|e| e["method"].as_str())
        .collect();
    assert!(
        methods.contains(&"notifications/resources/list_changed"),
        "requested list_changed must be delivered, got: {methods:?}"
    );
    assert!(
        methods.contains(&"notifications/resources/updated"),
        "watched resources/updated must be delivered, got: {methods:?}"
    );
    assert!(
        !methods.contains(&"notifications/prompts/list_changed"),
        "unrequested types must never be delivered"
    );

    for event in &delivered {
        assert_eq!(
            event["params"]["_meta"]["io.modelcontextprotocol/subscriptionId"], "7",
            "every notification on the stream must carry the subscription id: {event}"
        );
        if event["method"] == "notifications/resources/updated" {
            assert_eq!(
                event["params"]["uri"], "file:///watched.txt",
                "only watched URIs may produce resources/updated deliveries"
            );
        }
    }
}

#[tokio::test]
async fn listen_requires_sse_accept() {
    let url = start_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "subscriptions/listen")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "subscriptions/listen",
            "params": { "notifications": { "toolsListChanged": true }, "_meta": meta() }
        }))
        .send()
        .await
        .expect("listen without SSE accept");

    assert_eq!(
        resp.status(),
        400,
        "subscriptions/listen without Accept: text/event-stream must be rejected"
    );
}

#[tokio::test]
async fn listen_ack_omits_unsupported_types() {
    // Server WITHOUT prompts: promptsListChanged requested but unsupported →
    // omitted from the acknowledged filter.
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let server = McpServer::builder()
        .name("subscriptions-2026-noprompts")
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
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let resp = client
        .post(&url)
        .header("Accept", "text/event-stream")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "subscriptions/listen")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 3, "method": "subscriptions/listen",
            "params": {
                "notifications": { "toolsListChanged": true, "promptsListChanged": true },
                "_meta": meta()
            }
        }))
        .send()
        .await
        .expect("listen POST");
    assert_eq!(resp.status(), 200);

    let mut reader = SseReader::new(resp.bytes_stream());
    let ack = reader
        .next_json(Duration::from_secs(5))
        .await
        .expect("acknowledgement frame");
    assert_eq!(ack["method"], "notifications/subscriptions/acknowledged");
    assert_eq!(
        ack["params"]["notifications"]["toolsListChanged"], true,
        "supported requested type must be acknowledged"
    );
    assert!(
        ack["params"]["notifications"]
            .as_object()
            .unwrap()
            .get("promptsListChanged")
            .is_none(),
        "unsupported types must be omitted from the acknowledgement"
    );
}

#[tokio::test]
async fn resources_subscribe_capability_is_advertised_truthfully() {
    // The 2026 transport serves per-URI resources/updated via
    // subscriptions/listen, so a server WITH resources must advertise
    // resources.subscribe = true in server/discover.
    let url = start_server().await;
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "server/discover")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 21, "method": "server/discover",
            "params": { "_meta": meta() }
        }))
        .send()
        .await
        .expect("server/discover POST");
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.expect("json body");
    assert_eq!(
        body["result"]["capabilities"]["resources"]["subscribe"], true,
        "subscriptions/listen serves per-URI resource updates — the capability \
         must say so: {body}"
    );
}
