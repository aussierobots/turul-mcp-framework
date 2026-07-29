//! Wire-level acceptance for per-request log gating (2026-07-28).
//!
//! `RequestMetaObject.logLevel`: "If absent, the server MUST NOT send any
//! `notifications/message` notifications for this request. The client opts in
//! to log messages by explicitly setting a level. Replaces the former
//! `logging/setLevel` RPC."
//!
//! Built only under the 2026 feature; compiles to nothing under 2025-11-25.
#![cfg(feature = "protocol-2026-07-28")]
#![allow(deprecated)] // exercises the SEP-2577-deprecated logging surface

use std::time::Duration;

use futures::StreamExt;
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;

/// Emits one info-level `notifications/message` mid-execution, then returns.
#[derive(McpTool, Clone, Default)]
#[tool(name = "chatty", description = "Logs while working", output = String)]
struct ChattyTool {}

impl ChattyTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        if let Some(session) = session {
            session
                .notify_log(
                    turul_mcp_protocol::logging::LoggingLevel::Info,
                    serde_json::json!("working..."),
                    Some("chatty".to_string()),
                    None,
                )
                .await;
        }
        Ok("done".to_string())
    }
}

async fn start_server() -> String {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();

    let server = McpServer::builder()
        .name("loggate-2026-test")
        .version("0.4.0")
        .tool(ChattyTool::default())
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

/// POST a `chatty` tools/call with SSE framing; return every `data:` JSON
/// payload observed on the response stream.
async fn call_chatty(url: &str, log_level: Option<&str>) -> Vec<serde_json::Value> {
    let mut meta = serde_json::json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "test-client", "version": "1.0.0" },
        "io.modelcontextprotocol/clientCapabilities": {}
    });
    if let (Some(m), Some(level)) = (meta.as_object_mut(), log_level) {
        m.insert(
            "io.modelcontextprotocol/logLevel".to_string(),
            serde_json::json!(level),
        );
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        // Both media types accepted → tools/call gets the SSE framing, which
        // is where request-scoped notifications ride.
        .header("Accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "chatty")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": "chatty", "arguments": {}, "_meta": meta }
        }))
        .send()
        .await
        .expect("chatty POST");
    assert_eq!(resp.status(), 200);

    let mut events = Vec::new();
    let mut buffer = String::new();
    let mut stream = resp.bytes_stream();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, stream.next()).await {
            Ok(Some(Ok(chunk))) => buffer.push_str(&String::from_utf8_lossy(&chunk)),
            _ => break,
        }
        let mut done = false;
        while let Some(pos) = buffer.find("\n\n") {
            let event: String = buffer.drain(..pos + 2).collect();
            for line in event.lines() {
                if let Some(data) = line.strip_prefix("data: ")
                    && let Ok(json) = serde_json::from_str::<serde_json::Value>(data)
                {
                    // The final JSON-RPC response ends the request stream.
                    if json.get("result").is_some() || json.get("error").is_some() {
                        done = true;
                    }
                    events.push(json);
                }
            }
        }
        if done {
            break;
        }
    }
    events
}

#[tokio::test]
async fn message_notifications_require_a_declared_log_level() {
    let url = start_server().await;

    // Without logLevel in _meta the server MUST NOT emit notifications/message,
    // and it cannot: declaring neither a logLevel nor a progressToken opts the
    // request out of request-scoped notifications entirely, so the reply is a
    // single JSON object with no stream to carry one.
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "chatty")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": "chatty", "arguments": {}, "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientCapabilities": {}
            }}
        }))
        .send()
        .await
        .expect("POST");
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let body: serde_json::Value = resp.json().await.expect("json body");
    assert!(
        content_type.starts_with("application/json"),
        "no logLevel and no progressToken must be answered with a single JSON \
         object, got content-type {content_type:?}: {body}"
    );
    assert!(
        body["result"].is_object(),
        "the final response must still arrive: {body}"
    );
    assert!(
        body.get("method").is_none(),
        "a JSON reply carries the result only, never a notification: {body}"
    );

    // With logLevel "info": the info-level message must be delivered.
    let events = call_chatty(&url, Some("info")).await;
    assert!(
        events
            .iter()
            .any(|e| e["method"] == "notifications/message"),
        "declared logLevel must opt in to notifications/message, got: {events:?}"
    );
}

#[tokio::test]
async fn declared_level_is_the_severity_threshold() {
    let url = start_server().await;

    // The tool logs at info; a request declaring only "error" must not get it.
    let events = call_chatty(&url, Some("error")).await;
    assert!(
        !events
            .iter()
            .any(|e| e["method"] == "notifications/message"),
        "info-level message must be filtered below an 'error' threshold: {events:?}"
    );
}

/// Logging §logLevel: "If the io.modelcontextprotocol/logLevel value … is not
/// a recognized log level, the server SHOULD reject that request with …
/// -32602."
#[tokio::test]
async fn unrecognized_log_level_is_rejected_with_32602() {
    let url = start_server().await;
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "chatty")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 9, "method": "tools/call",
            "params": { "name": "chatty", "arguments": {}, "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientInfo": { "name": "t", "version": "1" },
                "io.modelcontextprotocol/clientCapabilities": {},
                "io.modelcontextprotocol/logLevel": "extra-loud"
            }}
        }))
        .send()
        .await
        .expect("POST");
    let body: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(
        body["error"]["code"], -32602,
        "unrecognized logLevel must be invalid params: {body}"
    );
}
