//! A progress notification references the token from the originating request.
//!
//! "Progress notifications MUST only reference tokens that were provided in an
//! active request" — so an arbitrary token is not correlation, it is noise a
//! client cannot match to its own call. The 2025-11-25 lane could not express
//! the compliant form at all: `SessionContext::progress_token()` and
//! `notify_request_progress()` were gated to 2026-07-28, leaving only
//! `notify_progress(arbitrary_string, ..)`. The concept is identical on both
//! specs — the token travels in `params._meta.progressToken` — only the
//! plumbing differed, because this spec snapshot types `CallToolParams::meta`
//! as an untyped map rather than a struct with a typed field.
//!
//! This is the strict progress E2E the 2025-11-25 baseline requires: at least
//! one progress event, and its token matching the request's.
#![cfg(feature = "protocol-2025-11-25")]

mod common;

use turul_mcp_server::prelude::*;

const TOKEN: &str = "progress-correlation-probe-1";

/// Emits progress against the REQUEST's token rather than a string of its own.
#[derive(Clone, Default)]
struct ProgressTool;

#[async_trait::async_trait]
impl McpTool for ProgressTool {
    async fn call(
        &self,
        _args: serde_json::Value,
        session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        let session = session.ok_or_else(|| McpError::tool_execution("session required"))?;
        let echoed = session.notify_request_progress(1.0, Some(2.0)).await;
        Ok(CallToolResult::success(vec![ToolResult::text(format!(
            "emitted={echoed}"
        ))]))
    }
}

impl turul_mcp_builders::traits::HasBaseMetadata for ProgressTool {
    fn name(&self) -> &str {
        "progress_probe"
    }
}
impl turul_mcp_builders::traits::HasDescription for ProgressTool {
    fn description(&self) -> Option<&str> {
        Some("Emits one progress notification against the request's token")
    }
}
impl turul_mcp_builders::traits::HasInputSchema for ProgressTool {
    fn input_schema(&self) -> &turul_mcp_protocol::tools::ToolSchema {
        use std::sync::OnceLock;
        static S: OnceLock<turul_mcp_protocol::tools::ToolSchema> = OnceLock::new();
        S.get_or_init(turul_mcp_protocol::tools::ToolSchema::object)
    }
}
impl turul_mcp_builders::traits::HasOutputSchema for ProgressTool {}
impl turul_mcp_builders::traits::HasAnnotations for ProgressTool {}
impl turul_mcp_builders::traits::HasToolMeta for ProgressTool {}
impl turul_mcp_builders::traits::HasExecution for ProgressTool {}
impl turul_mcp_builders::traits::HasIcons for ProgressTool {}

/// SSE frames carry one JSON object per `data:` line.
fn data_payloads(body: &str) -> Vec<serde_json::Value> {
    body.lines()
        .filter_map(|l| l.strip_prefix("data: "))
        .filter_map(|d| serde_json::from_str(d).ok())
        .collect()
}

#[tokio::test]
async fn a_progress_notification_carries_the_requests_own_token() {
    let reserved = common::reserve_port().await;
    let port = reserved.port;

    let server = McpServer::builder()
        .name("progress-correlation")
        .version("0.4.0")
        .tool(ProgressTool)
        .bind_address(format!("127.0.0.1:{port}").parse().unwrap())
        .build()
        .expect("build 2025-11-25 server");

    tokio::spawn(async move {
        server.run().await.ok();
    });

    let url = format!("http://127.0.0.1:{port}/mcp");
    let client = reqwest::Client::new();
    for _ in 0..100 {
        if client.get(&url).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    drop(reserved);

    let init = client
        .post(&url)
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": { "name": "progress-probe", "version": "0.4.0" }
            }
        }))
        .send()
        .await
        .expect("initialize");
    let session = init
        .headers()
        .get("mcp-session-id")
        .expect("session id")
        .to_str()
        .unwrap()
        .to_string();

    let accepted = client
        .post(&url)
        .header("Accept", "application/json")
        .header("Mcp-Session-Id", &session)
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "method": "notifications/initialized"
        }))
        .send()
        .await
        .expect("initialized");
    assert_eq!(accepted.status(), 202);

    // Accept: text/event-stream so progress and the result arrive on one
    // stream — no GET/POST ordering race to lose the notification to.
    let body = client
        .post(&url)
        .header("Accept", "text/event-stream")
        .header("Mcp-Session-Id", &session)
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": {
                "name": "progress_probe",
                "arguments": {},
                "_meta": { "progressToken": TOKEN }
            }
        }))
        .send()
        .await
        .expect("tools/call")
        .text()
        .await
        .expect("sse body");

    let frames = data_payloads(&body);
    let progress: Vec<&serde_json::Value> = frames
        .iter()
        .filter(|f| f["method"] == "notifications/progress")
        .collect();

    assert!(
        !progress.is_empty(),
        "expected at least one notifications/progress frame, got: {body}"
    );
    for p in &progress {
        assert_eq!(
            p["params"]["progressToken"], TOKEN,
            "progress must reference the token the request supplied, not one of \
             the tool's own choosing: {p}"
        );
    }

    // Correlate against the tool response on the same stream: the request that
    // carried the token is the one that answered.
    let result = frames
        .iter()
        .find(|f| f["id"] == 2 && f.get("result").is_some())
        .unwrap_or_else(|| panic!("no tools/call result on the stream: {body}"));
    assert!(
        result["result"]["content"][0]["text"]
            .as_str()
            .is_some_and(|t| t.contains("emitted=true")),
        "the tool must report that it found a request token to echo: {result}"
    );
}
