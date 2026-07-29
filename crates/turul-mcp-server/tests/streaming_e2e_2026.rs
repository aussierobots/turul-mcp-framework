//! Wire-grammar acceptance for the streaming response path (2026-07-28).
//!
//! The other 2026 streaming suites read `data:` lines leniently and assert on
//! the JSON they carry. That leaves the framing itself unasserted: a server
//! could emit malformed field lines, omit the blank-line terminator, or cut the
//! stream mid-frame and every one of those tests would still pass, because they
//! only look at payloads they managed to parse.
//!
//! This suite asserts the bytes. It captures the whole response body and checks
//! it against the Server-Sent Events grammar an independent client must parse:
//! events separated by a blank line, every non-empty line a known field, every
//! `data:` payload a complete JSON-RPC message, and the stream ending on a
//! frame boundary rather than part way through one.
//!
//! Built only under the 2026 feature; compiles to nothing under 2025-11-25.
#![cfg(feature = "protocol-2026-07-28")]

mod common;

use std::time::Duration;

use futures::StreamExt;
use turul_http_mcp_server::notification_bridge::SharedNotificationBroadcaster;
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;
use turul_rpc::JsonRpcNotification;

/// Emits progress mid-execution so the stream carries more than one frame.
#[derive(McpTool, Clone, Default)]
#[tool(name = "streamer", description = "Emits progress then completes", output = String)]
struct StreamerTool {}

impl StreamerTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        if let Some(session) = session {
            session.notify_request_progress(0.25, Some(1.0)).await;
            session.notify_request_progress(0.75, Some(1.0)).await;
        }
        Ok("streamed".to_string())
    }
}

/// Pushes one `tools/list_changed` through the server-wide broadcast pipeline,
/// so an open listen stream has a delivered frame to inspect.
#[derive(McpTool, Clone, Default)]
#[tool(name = "emit", description = "Broadcast a tools list change", output = String)]
struct EmitTool {}

impl EmitTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        let session = session.ok_or_else(|| McpError::tool_execution("session required"))?;
        let broadcaster = session
            .broadcaster
            .as_ref()
            .and_then(|any| any.downcast_ref::<SharedNotificationBroadcaster>())
            .ok_or_else(|| McpError::tool_execution("broadcaster required"))?
            .clone();
        let _ = broadcaster
            .broadcast_to_all_sessions(JsonRpcNotification::new_no_params(
                "notifications/tools/list_changed".to_string(),
            ))
            .await;
        Ok("emitted".to_string())
    }
}

async fn start_server() -> String {
    let reserved = common::reserve_port().await;
    let port = reserved.port;

    let server = McpServer::builder()
        .name("streaming-e2e-2026")
        .version("0.4.0")
        .tool(StreamerTool::default())
        .tool(EmitTool::default())
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

fn meta(progress_token: Option<&str>) -> serde_json::Value {
    let mut meta = serde_json::json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "test-client", "version": "1.0.0" },
        "io.modelcontextprotocol/clientCapabilities": {}
    });
    if let (Some(m), Some(token)) = (meta.as_object_mut(), progress_token) {
        m.insert("progressToken".to_string(), serde_json::json!(token));
    }
    meta
}

fn call_body(progress_token: Option<&str>) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": { "name": "streamer", "arguments": {}, "_meta": meta(progress_token) }
    })
}

async fn post_call(url: &str, progress_token: Option<&str>) -> reqwest::Response {
    reqwest::Client::new()
        .post(url)
        .header("Accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "streamer")
        .json(&call_body(progress_token))
        .send()
        .await
        .expect("tools/call POST")
}

/// Drains a response body to EOF (or the deadline) and returns it verbatim.
///
/// Deliberately not incremental: the assertions here are about the shape of the
/// whole byte sequence, including how it ends.
async fn drain_body(resp: reqwest::Response) -> String {
    let mut raw = Vec::new();
    let mut stream = resp.bytes_stream();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, stream.next()).await {
            Ok(Some(Ok(chunk))) => raw.extend_from_slice(&chunk),
            // Stream end, transport error, or timeout — all terminate the read.
            _ => break,
        }
    }
    String::from_utf8(raw).expect("SSE bodies are UTF-8")
}

/// Splits a captured body into `\n\n`-terminated event blocks, leaving any
/// unterminated tail as the second element. A well-formed stream has an empty
/// tail; a stream cut mid-frame does not.
fn split_events(body: &str) -> (Vec<&str>, &str) {
    let mut events = Vec::new();
    let mut rest = body;
    while let Some(pos) = rest.find("\n\n") {
        events.push(&rest[..pos]);
        rest = &rest[pos + 2..];
    }
    (events, rest)
}

fn data_payloads(event: &str) -> Vec<&str> {
    event
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .collect()
}

/// Accumulates into `buffer` until it holds `want` terminated frames, or the
/// deadline passes. A `subscriptions/listen` stream never ends on its own, so
/// draining it to EOF would hang.
async fn read_frames<S>(
    stream: &mut S,
    buffer: &mut String,
    want: usize,
    deadline: tokio::time::Instant,
) where
    S: futures::Stream<Item = reqwest::Result<bytes::Bytes>> + Unpin,
{
    while buffer.matches("\n\n").count() < want {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return;
        }
        match tokio::time::timeout(remaining, stream.next()).await {
            Ok(Some(Ok(chunk))) => buffer.push_str(&String::from_utf8_lossy(&chunk)),
            _ => return,
        }
    }
}

/// Response headers must describe a stream, not a buffered document: an SSE
/// body has no known length, and a caching intermediary that held it would
/// stall every notification behind the final result.
#[tokio::test]
async fn sse_response_headers_declare_an_unbuffered_stream() {
    let url = start_server().await;
    let resp = post_call(&url, Some("tok")).await;
    assert_eq!(resp.status(), 200);

    let headers = resp.headers().clone();
    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert!(
        content_type.starts_with("text/event-stream"),
        "streaming replies are text/event-stream, got {content_type:?}"
    );

    let cache_control = headers
        .get("cache-control")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert!(
        cache_control.contains("no-cache"),
        "an SSE stream must not be cached, got Cache-Control: {cache_control:?}"
    );

    assert!(
        headers.get("content-length").is_none(),
        "a stream has no predeclared length, got Content-Length: {:?}",
        headers.get("content-length")
    );
}

/// Every line of every frame must be something an SSE parser recognises.
/// A stray unprefixed line does not error in a lenient reader — it is silently
/// dropped, which is exactly how a malformed frame reaches production unnoticed.
#[tokio::test]
async fn sse_body_matches_the_event_stream_grammar() {
    let url = start_server().await;
    let body = drain_body(post_call(&url, Some("tok")).await).await;
    let (events, tail) = split_events(&body);

    assert!(
        !events.is_empty(),
        "expected at least one terminated event, got body {body:?}"
    );

    for event in &events {
        for line in event.lines() {
            let known = line.is_empty()
                || line.starts_with(':')
                || line.starts_with("data:")
                || line.starts_with("event:")
                || line.starts_with("id:")
                || line.starts_with("retry:");
            assert!(
                known,
                "unrecognised SSE field line {line:?} in event {event:?}"
            );
        }

        let payloads = data_payloads(event);
        assert_eq!(
            payloads.len(),
            1,
            "each frame carries exactly one JSON-RPC message: {event:?}"
        );
        let json: serde_json::Value =
            serde_json::from_str(payloads[0]).expect("every data: payload is complete JSON");
        assert_eq!(
            json["jsonrpc"], "2.0",
            "frames on the wire are JSON-RPC envelopes: {json}"
        );
    }

    assert!(
        tail.is_empty(),
        "the stream ended part way through a frame; trailing bytes: {tail:?}"
    );
}

/// The result frame terminates the stream. If notifications could follow it,
/// a client that stops reading at the result — the obvious implementation —
/// would lose them, and one that keeps reading would hang.
#[tokio::test]
async fn the_result_frame_is_last_and_closes_the_stream() {
    let url = start_server().await;
    let body = drain_body(post_call(&url, Some("tok")).await).await;
    let (events, _) = split_events(&body);

    let parsed: Vec<serde_json::Value> = events
        .iter()
        .map(|e| serde_json::from_str(data_payloads(e)[0]).expect("json"))
        .collect();

    let result_idx = parsed
        .iter()
        .position(|j| j.get("result").is_some() || j.get("error").is_some())
        .unwrap_or_else(|| panic!("no result frame in {parsed:?}"));

    assert_eq!(
        result_idx,
        parsed.len() - 1,
        "nothing may follow the result frame: {parsed:?}"
    );
    assert!(
        parsed[..result_idx]
            .iter()
            .all(|j| j["method"] == "notifications/progress"),
        "only request-scoped notifications precede the result: {parsed:?}"
    );
    assert_eq!(
        result_idx, 2,
        "both progress notifications must arrive before the result, not be \
         collapsed or dropped: {parsed:?}"
    );
}

/// Ordering is the property a buffered implementation silently breaks: it can
/// emit the same frames and still deliver them all after the work finished.
#[tokio::test]
async fn progress_frames_carry_increasing_values_before_the_result() {
    let url = start_server().await;
    let body = drain_body(post_call(&url, Some("tok")).await).await;
    let (events, _) = split_events(&body);

    let progress: Vec<f64> = events
        .iter()
        .filter_map(|e| serde_json::from_str::<serde_json::Value>(data_payloads(e)[0]).ok())
        .filter(|j| j["method"] == "notifications/progress")
        .filter_map(|j| j["params"]["progress"].as_f64())
        .collect();

    assert_eq!(
        progress,
        vec![0.25, 0.75],
        "progress must arrive in emission order with values intact"
    );
}

/// The non-streaming counterpart, asserted here so the two framings stay
/// distinguishable on the wire rather than by inspection of the payload.
#[tokio::test]
async fn json_replies_are_a_single_object_with_no_event_framing() {
    let url = start_server().await;
    let resp = post_call(&url, None).await;
    assert_eq!(resp.status(), 200);

    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(
        content_type.starts_with("application/json"),
        "no progressToken means a buffered JSON reply, got {content_type:?}"
    );

    let body = drain_body(resp).await;
    assert!(
        !body.contains("data: ") && !body.contains("event: "),
        "a JSON reply must carry no SSE framing: {body:?}"
    );

    let json: serde_json::Value =
        serde_json::from_str(&body).expect("the whole body is one JSON document");
    assert!(json["result"].is_object(), "expected a result: {json}");
}

/// `subscriptions/listen` is a long-lived stream rather than a request-scoped
/// one, and its frames are labelled differently from the request-scoped path
/// asserted above: they name an event type, and delivered notifications carry
/// an `id:` cursor. Nothing else in the suite looks at those field lines — the
/// subscriptions tests read the JSON and ignore the framing around it.
///
/// The acknowledgement is deliberately not asserted to carry `id:`. It opens
/// the stream rather than being a delivered event, and the server emits no
/// cursor on it.
#[tokio::test]
async fn listen_frames_are_labelled_and_delivered_events_carry_a_cursor() {
    let url = start_server().await;
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Accept", "text/event-stream")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "subscriptions/listen")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": "sub-1", "method": "subscriptions/listen",
            "params": {
                "notifications": { "toolsListChanged": true },
                "_meta": meta(None)
            }
        }))
        .send()
        .await
        .expect("subscriptions/listen POST");
    assert_eq!(resp.status(), 200);

    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    read_frames(&mut stream, &mut buffer, 1, deadline).await;

    let (events, _) = split_events(&buffer);
    let ack = events
        .first()
        .unwrap_or_else(|| panic!("no acknowledgement frame in {buffer:?}"));
    assert!(
        ack.lines().any(|l| l == "event: message"),
        "listen frames name their event type: {ack:?}"
    );
    let json: serde_json::Value =
        serde_json::from_str(data_payloads(ack)[0]).expect("ack payload is JSON");
    assert_eq!(
        json["method"], "notifications/subscriptions/acknowledged",
        "the first frame acknowledges the subscription: {json}"
    );

    // Push a notification onto the open stream, then inspect its framing.
    client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "emit")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": { "name": "emit", "arguments": {}, "_meta": meta(None) }
        }))
        .send()
        .await
        .expect("emit POST");

    read_frames(&mut stream, &mut buffer, 2, deadline).await;
    let (events, _) = split_events(&buffer);
    let delivered = events
        .get(1)
        .unwrap_or_else(|| panic!("no delivered frame after the ack in {buffer:?}"));

    assert!(
        delivered.lines().any(|l| l.starts_with("id: ")),
        "a delivered notification carries a cursor: {delivered:?}"
    );
    assert!(
        delivered.lines().any(|l| l == "event: message"),
        "a delivered notification names its event type: {delivered:?}"
    );
    let json: serde_json::Value =
        serde_json::from_str(data_payloads(delivered)[0]).expect("delivered payload is JSON");
    assert_eq!(json["method"], "notifications/tools/list_changed");
}
