//! Wire-level acceptance for request-scoped progress on the 2026 path.
//!
//! Progress §Behavior: "Progress notifications MUST only reference tokens
//! that: Were provided in an active request; Are associated with an
//! in-progress operation." The caller opts in via `_meta.progressToken`;
//! a request without one gets no `notifications/progress`, and the echoed
//! token preserves the caller's JSON type (string or number).
//!
//! Built only under the 2026 feature; compiles to nothing under 2025-11-25.
#![cfg(feature = "protocol-2026-07-28")]

use std::time::Duration;

use futures::StreamExt;
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;

/// Emits one request-scoped progress notification mid-execution.
#[derive(McpTool, Clone, Default)]
#[tool(name = "worker", description = "Works with progress", output = String)]
struct WorkerTool {}

impl WorkerTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        if let Some(session) = session {
            session.notify_request_progress(0.5, Some(1.0)).await;
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
        .name("progress-2026-test")
        .version("0.4.0")
        .tool(WorkerTool::default())
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

/// POST a `worker` tools/call with SSE framing; return every `data:` JSON
/// payload observed on the response stream.
async fn call_worker(
    url: &str,
    progress_token: Option<serde_json::Value>,
) -> Vec<serde_json::Value> {
    let mut meta = serde_json::json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "test-client", "version": "1.0.0" },
        "io.modelcontextprotocol/clientCapabilities": {}
    });
    if let (Some(m), Some(token)) = (meta.as_object_mut(), progress_token) {
        m.insert("progressToken".to_string(), token);
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .header("Accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "worker")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": "worker", "arguments": {}, "_meta": meta }
        }))
        .send()
        .await
        .expect("worker POST");
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

fn progress_notifications(events: &[serde_json::Value]) -> Vec<&serde_json::Value> {
    events
        .iter()
        .filter(|e| e["method"] == "notifications/progress")
        .collect()
}

#[tokio::test]
async fn progress_echoes_the_request_string_token() {
    let url = start_server().await;
    let events = call_worker(&url, Some(serde_json::json!("tok-1"))).await;
    let progress = progress_notifications(&events);
    assert!(
        !progress.is_empty(),
        "a declared progressToken must opt in to notifications/progress: {events:?}"
    );
    assert_eq!(
        progress[0]["params"]["progressToken"],
        serde_json::json!("tok-1"),
        "the notification must reference the REQUEST's token: {progress:?}"
    );
    assert_eq!(progress[0]["params"]["progress"], serde_json::json!(0.5));
    assert_eq!(progress[0]["params"]["total"], serde_json::json!(1.0));
}

#[tokio::test]
async fn progress_preserves_a_numeric_token() {
    let url = start_server().await;
    let events = call_worker(&url, Some(serde_json::json!(7))).await;
    let progress = progress_notifications(&events);
    assert!(!progress.is_empty(), "{events:?}");
    assert_eq!(
        progress[0]["params"]["progressToken"],
        serde_json::json!(7),
        "a numeric token must round-trip as a JSON number: {progress:?}"
    );
}

#[tokio::test]
async fn no_token_means_no_progress_notifications() {
    let url = start_server().await;
    let events = call_worker(&url, None).await;
    assert!(
        !events.is_empty(),
        "the final response must arrive on the stream"
    );
    assert!(
        progress_notifications(&events).is_empty(),
        "MUST only reference tokens provided in an active request — no token, \
         no notifications/progress: {events:?}"
    );
}

/// A numeric progressToken must survive the typed params parse.
#[test]
fn numeric_token_parses_into_call_params() {
    let params = serde_json::json!({
        "name": "worker", "arguments": {},
        "_meta": {
            "io.modelcontextprotocol/protocolVersion": "2026-07-28",
            "io.modelcontextprotocol/clientInfo": { "name": "t", "version": "1" },
            "io.modelcontextprotocol/clientCapabilities": {},
            "progressToken": 7
        }
    });
    let typed: turul_mcp_protocol::tools::CallToolRequestParams =
        serde_json::from_value(params).expect("parse");
    assert!(
        typed.meta.progress_token.is_some(),
        "numeric progressToken must survive the typed parse: {:?}",
        typed.meta
    );
}
