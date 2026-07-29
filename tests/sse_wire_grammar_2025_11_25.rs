//! Wire-grammar acceptance for the SSE streaming paths on MCP 2025-11-25.
//!
//! `streaming_e2e_2026.rs` (crates/turul-mcp-server/tests/) asserts the SSE
//! grammar an independent client must parse: events separated by a blank
//! line, every non-empty line a known field, every `data:` payload a
//! complete JSON-RPC message, and the stream ending on a frame boundary.
//! The equivalent 2025-11-25 suites (`sse_progress_delivery.rs`,
//! `e2e_sse_notification_roundtrip.rs`) only ever do
//! `line.strip_prefix("data: ")` — a stray unrecognised field line, a
//! missing blank-line terminator, or a stream cut mid-frame would pass
//! silently through all of them.
//!
//! `streamable_http_e2e.rs` already has a `parse_sse_events` helper, but it
//! is itself lenient: unrecognised (non-colon) lines are silently dropped
//! rather than failing the test, and a stream that ends without a trailing
//! blank line is treated as a valid final event rather than a truncation.
//!
//! This suite closes both gaps for the 2025-11-25 lane, on real server
//! responses (`turul-mcp-server` built with `protocol-2025-11-25`):
//!
//! - `post_sse_stream_matches_the_event_stream_grammar_and_ends_on_a_frame_boundary`
//!   drains a real streaming `tools/call` POST response and checks every
//!   byte against the grammar.
//! - `get_sse_stream_replays_events_after_last_event_id_with_valid_frames`
//!   covers what 2026 removed and this lane still has: the standalone GET
//!   SSE stream and `Last-Event-ID` resumption. It asserts that a live
//!   delivered event and a replayed (post-reconnect) event both carry a
//!   well-formed `id:` cursor, and that the replayed cursor is strictly
//!   after the one supplied in `Last-Event-ID`.
//!
//! `client_streaming_test.rs` was not touched: its `test_handle_byte_stream`
//! calls exercise the client's `application/json` chunked-JSON parser
//! (`HttpTransport::handle_byte_stream`), a real but distinct wire path used
//! only when the server answers `Content-Type: application/json`. It never
//! emits `data:`/`event:`/`id:` lines, so there is no SSE grammar to assert
//! there — porting the 2026 assumption onto it would have asserted a
//! framing that file's code path does not use.

use std::time::Duration;

use futures::StreamExt;
use mcp_e2e_shared::{McpTestClient, TestServerManager};
use serde_json::{Value, json};
use serial_test::serial;
use tokio::time::Instant;

/// Every non-empty SSE field line this server emits (see
/// `turul-mcp-session-storage::SseEvent::format`): `id:`, `event:`, `data:`,
/// `retry:`, or a comment line (`:`) for keepalives. Anything else is a
/// framing defect, not a line a lenient parser gets to ignore.
fn known_line(line: &str) -> bool {
    line.is_empty()
        || line.starts_with(':')
        || line.starts_with("data:")
        || line.starts_with("event:")
        || line.starts_with("id:")
        || line.starts_with("retry:")
}

/// Splits a captured body into `\n\n`-terminated event blocks, leaving any
/// unterminated tail as the second element. A well-formed stream has an
/// empty tail; a stream cut mid-frame does not.
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

fn event_id(event: &str) -> Option<u64> {
    event
        .lines()
        .find_map(|line| line.strip_prefix("id: "))
        .and_then(|id| id.parse().ok())
}

/// The first terminated event in `buffer` carrying a `data:` payload — i.e.
/// the first non-keepalive frame. Comment-only keepalive frames can precede
/// it (see `assert_wire_grammar`), so index 0 of the raw split is not
/// reliably the message a caller is looking for.
fn first_data_event(buffer: &str) -> &str {
    let (events, _) = split_events(buffer);
    events
        .into_iter()
        .find(|event| !data_payloads(event).is_empty())
        .unwrap_or_else(|| panic!("no non-keepalive frame found in {buffer:?}"))
}

/// Asserts full SSE grammar compliance for every terminated event in `body`,
/// and that the stream ended on a frame boundary. Comment-only keepalive
/// frames (`: keepalive`) are valid grammar but carry no JSON-RPC message —
/// `tokio::time::interval`'s first tick fires immediately, so one can appear
/// before any real event on a freshly opened GET stream. Returns the parsed
/// JSON-RPC payloads of the non-keepalive frames, in wire order.
fn assert_wire_grammar(body: &str) -> Vec<Value> {
    let (events, tail) = split_events(body);
    assert!(
        !events.is_empty(),
        "expected at least one terminated SSE event, got body {body:?}"
    );

    let mut parsed = Vec::new();
    for event in &events {
        for line in event.lines() {
            assert!(
                known_line(line),
                "unrecognised SSE field line {line:?} in event {event:?}"
            );
        }

        let payloads = data_payloads(event);
        if payloads.is_empty() {
            assert!(
                event.lines().all(|line| line.starts_with(':')),
                "a frame with no data: payload must be comment-only (keepalive): {event:?}"
            );
            continue;
        }
        assert_eq!(
            payloads.len(),
            1,
            "each non-keepalive frame carries exactly one JSON-RPC message: {event:?}"
        );
        let json: Value =
            serde_json::from_str(payloads[0]).expect("every data: payload is complete JSON");
        assert_eq!(
            json["jsonrpc"], "2.0",
            "frames on the wire are JSON-RPC envelopes: {json}"
        );
        parsed.push(json);
    }

    assert!(
        tail.is_empty(),
        "the stream ended part way through a frame; trailing bytes: {tail:?}"
    );

    parsed
}

/// Number of terminated frames in `buffer` that carry a `data:` payload,
/// i.e. excluding comment-only keepalive frames.
fn count_data_frames(buffer: &str) -> usize {
    let (events, _) = split_events(buffer);
    events
        .iter()
        .filter(|event| !data_payloads(event).is_empty())
        .count()
}

/// Accumulates into `buffer` until it holds `want` terminated frames that
/// carry a JSON-RPC message, or the deadline passes. A GET SSE stream never
/// ends on its own, so draining it to EOF would hang; keepalive comment
/// frames are excluded from the count since they carry no message and can
/// arrive at any time (the keepalive interval's first tick is immediate).
async fn read_frames<S>(stream: &mut S, buffer: &mut String, want: usize, deadline: Instant)
where
    S: futures::Stream<Item = reqwest::Result<bytes::Bytes>> + Unpin,
{
    while count_data_frames(buffer) < want {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return;
        }
        match tokio::time::timeout(remaining, stream.next()).await {
            Ok(Some(Ok(chunk))) => buffer.push_str(&String::from_utf8_lossy(&chunk)),
            _ => return,
        }
    }
}

/// The request-scoped streaming path: progress notifications followed by the
/// tool result, all on one POST response. Every frame — including the final
/// result — must obey the grammar, and the response must end exactly on a
/// frame boundary rather than leaving a partial trailer.
#[tokio::test]
#[serial]
async fn post_sse_stream_matches_the_event_stream_grammar_and_ends_on_a_frame_boundary() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = match TestServerManager::start_tools_server().await {
        Ok(server) => server,
        Err(e) => {
            println!("Skipping — failed to start test server (sandboxed environment?): {e}");
            return;
        }
    };

    let mut client = McpTestClient::new(server.port());
    client
        .initialize_with_capabilities(json!({ "tools": { "listChanged": false } }))
        .await
        .expect("initialize");
    client
        .send_initialized_notification()
        .await
        .expect("notifications/initialized");

    let response = client
        .call_tool_with_sse("progress_tracker", json!({ "duration": 0.3, "steps": 2 }))
        .await
        .expect("tools/call with SSE Accept");
    assert_eq!(response.status(), 200);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok()),
        Some("text/event-stream")
    );

    let body = tokio::time::timeout(Duration::from_secs(10), response.text())
        .await
        .expect("stream did not end within the deadline")
        .expect("readable body");

    let frames = assert_wire_grammar(&body);

    let result_idx = frames
        .iter()
        .position(|j| j.get("result").is_some() || j.get("error").is_some())
        .unwrap_or_else(|| panic!("no result frame in {frames:?}"));
    assert_eq!(
        result_idx,
        frames.len() - 1,
        "nothing may follow the result frame: {frames:?}"
    );
    assert!(
        frames[..result_idx]
            .iter()
            .all(|j| j["method"] == "notifications/progress"),
        "only progress notifications precede the result: {frames:?}"
    );
    assert!(
        result_idx > 0,
        "progress_tracker(duration=0.3, steps=2) must emit at least one progress \
         notification before the result: {frames:?}"
    );

    let progress_values: Vec<f64> = frames[..result_idx]
        .iter()
        .filter_map(|j| j["params"]["progress"].as_f64())
        .collect();
    assert_eq!(
        progress_values.len(),
        result_idx,
        "every notification before the result is progress with a numeric value: {frames:?}"
    );
    for pair in progress_values.windows(2) {
        assert!(
            pair[0] <= pair[1],
            "progress must not regress across frames: {progress_values:?}"
        );
    }
}

/// Standalone GET SSE stream + `Last-Event-ID` resumption — surface removed
/// entirely in 2026-07-28, still load-bearing here. A delivered live event
/// and a replayed post-reconnect event must both be well-formed frames, and
/// the replayed cursor must sit strictly after the one the client supplied.
#[tokio::test]
#[serial]
async fn get_sse_stream_replays_events_after_last_event_id_with_valid_frames() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = match TestServerManager::start_dynamic_tools_server().await {
        Ok(server) => server,
        Err(e) => {
            println!("Skipping — failed to start test server (sandboxed environment?): {e}");
            return;
        }
    };

    let mut client = McpTestClient::new(server.port());
    client
        .initialize_with_capabilities(json!({ "tools": { "listChanged": true } }))
        .await
        .expect("initialize");
    client
        .send_initialized_notification()
        .await
        .expect("notifications/initialized");
    let session_id = client
        .session_id()
        .expect("initialize must return a session id")
        .clone();

    let base_url = format!("http://127.0.0.1:{}/mcp", server.port());
    let http = reqwest::Client::new();

    // Open the standalone GET SSE stream, then trigger a broadcast
    // (`deactivate_multiply` flips the dynamic tool set and fires
    // `notifications/tools/list_changed` to every session) while connected,
    // so it is delivered live rather than only ever replayed.
    let get1 = http
        .get(&base_url)
        .header("Accept", "text/event-stream")
        .header("MCP-Protocol-Version", "2025-11-25")
        .header("Mcp-Session-Id", &session_id)
        .send()
        .await
        .expect("GET SSE stream open");
    assert_eq!(get1.status(), 200);
    assert_eq!(
        get1.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok()),
        Some("text/event-stream")
    );

    let mut stream1 = get1.bytes_stream();
    let mut buf1 = String::new();

    let deactivate = client
        .call_tool("deactivate_multiply", json!({}))
        .await
        .expect("deactivate_multiply call");
    assert!(
        deactivate.contains_key("result"),
        "deactivate_multiply must succeed: {deactivate:?}"
    );

    read_frames(
        &mut stream1,
        &mut buf1,
        1,
        Instant::now() + Duration::from_secs(10),
    )
    .await;

    let live_frames = assert_wire_grammar(&buf1);
    assert_eq!(
        live_frames.len(),
        1,
        "expected exactly one live-delivered event on the open GET stream: {buf1:?}"
    );
    assert_eq!(
        live_frames[0]["method"], "notifications/tools/list_changed",
        "the broadcast tool-registry change must be what gets delivered: {live_frames:?}"
    );
    let live_event = first_data_event(&buf1);
    let first_id = event_id(live_event)
        .unwrap_or_else(|| panic!("live-delivered event carries no id: cursor: {live_event:?}"));

    // Disconnect, fire a second broadcast while nobody is listening, then
    // reconnect with Last-Event-ID set to the first event's cursor. Per the
    // MCP resumability contract the server MUST replay
    // everything strictly after that id on the resumed stream.
    drop(stream1);

    let activate = client
        .call_tool("activate_multiply", json!({}))
        .await
        .expect("activate_multiply call");
    assert!(
        activate.contains_key("result"),
        "activate_multiply must succeed: {activate:?}"
    );

    let get2 = http
        .get(&base_url)
        .header("Accept", "text/event-stream")
        .header("MCP-Protocol-Version", "2025-11-25")
        .header("Mcp-Session-Id", &session_id)
        .header("Last-Event-ID", first_id.to_string())
        .send()
        .await
        .expect("GET SSE resumption request");
    assert_eq!(
        get2.status(),
        200,
        "resumption with Last-Event-ID must succeed"
    );
    assert_eq!(
        get2.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok()),
        Some("text/event-stream")
    );

    let mut stream2 = get2.bytes_stream();
    let mut buf2 = String::new();
    read_frames(
        &mut stream2,
        &mut buf2,
        1,
        Instant::now() + Duration::from_secs(10),
    )
    .await;

    let replayed_frames = assert_wire_grammar(&buf2);
    assert!(
        !replayed_frames.is_empty(),
        "expected the activate_multiply broadcast to be replayed after Last-Event-ID {first_id}, \
         got body {buf2:?}"
    );
    assert_eq!(
        replayed_frames[0]["method"], "notifications/tools/list_changed",
        "the replayed frame must be the broadcast fired while disconnected: {replayed_frames:?}"
    );
    let replayed_event = first_data_event(&buf2);
    let replayed_id = event_id(replayed_event)
        .unwrap_or_else(|| panic!("replayed event carries no id: cursor: {replayed_event:?}"));
    assert!(
        replayed_id > first_id,
        "replay must be strictly after Last-Event-ID {first_id}, got {replayed_id}"
    );
}
