//! Regression tests for SSE GET 4xx terminal handling.
//!
//! Bug context: prior to v0.3.38, the streamable HTTP transport's SSE GET
//! listener treated every non-2xx response as retriable with a static 5s
//! sleep. Against an MCP server with `strict_lifecycle(true)`, a GET issued
//! after session termination would return 400 forever, producing a hot loop.
//!
//! These tests lock in:
//!   1. HTTP 4xx on the SSE GET path is terminal — listener exits, no retry.
//!   2. The cached `Mcp-Session-Id` is cleared on terminal 4xx, so the
//!      caller's next initialize POST goes out without a stale header.
//!   3. HTTP 5xx remains transient — listener does not exit on first 503.
//!   4. Stateless mode (no session header at all) still works — guards
//!      against accidental "abort if session_id is None" pre-flight.

use std::time::Duration;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use turul_mcp_client::config::ConnectionConfig;
use turul_mcp_client::transport::ServerEvent;
use turul_mcp_client::transport::Transport;
use turul_mcp_client::transport::http::HttpTransport;

fn json_rpc_ok() -> serde_json::Value {
    serde_json::json!({ "jsonrpc": "2.0", "id": "req_0", "result": {} })
}

fn ping_request() -> serde_json::Value {
    serde_json::json!({ "jsonrpc": "2.0", "id": "req_0", "method": "ping", "params": {} })
}

/// 1. Terminal 4xx: listener exits, no retry, session cache cleared.
#[tokio::test]
async fn test_sse_get_400_terminates_listener_and_clears_session() {
    let mock_server = MockServer::start().await;

    // GET /mcp returns 400 — should be hit at most once and then never again.
    Mock::given(method("GET"))
        .and(path("/mcp"))
        .respond_with(ResponseTemplate::new(400))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Subsequent POST must NOT carry an Mcp-Session-Id header (cache cleared).
    // wiremock's `header_exists` would assert presence; we want absence, which
    // we express by matching only requests that do NOT have the header via
    // a custom matcher. Simpler: match on absence using `BodyMatcher`-style
    // negation isn't built-in, so we install two mocks and rely on `expect`:
    // the "with header" mock expects 0 hits, the "any POST" mock expects 1.
    Mock::given(method("POST"))
        .and(path("/mcp"))
        .and(wiremock::matchers::header_exists("Mcp-Session-Id"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&mock_server)
        .await;
    Mock::given(method("POST"))
        .and(path("/mcp"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(json_rpc_ok()),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = ConnectionConfig::default();
    let transport =
        HttpTransport::with_config(&format!("{}/mcp", mock_server.uri()), &config).unwrap();
    transport.connect().await.unwrap();

    // Pre-populate a stale session ID — the bug is that this would survive
    // the listener's terminal 4xx and contaminate the next POST.
    transport.set_session_id("stale-session-id".to_string());

    let mut rx = transport.start_event_listener().await.unwrap();

    // Listener should emit one Error event then close the channel.
    let evt = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("listener must emit an event within 2s")
        .expect("channel must yield Some(_) before closing");
    match evt {
        ServerEvent::Error(msg) => {
            assert!(
                msg.contains("400"),
                "error must mention HTTP 400, got: {msg}"
            );
            assert!(
                msg.to_lowercase().contains("listener exiting"),
                "error must signal listener exit, got: {msg}"
            );
        }
        other => panic!("expected ServerEvent::Error, got {other:?}"),
    }

    // Note: rx does NOT close because `start_event_listener` keeps a sender
    // clone in `self.event_sender` (http.rs around line 796). The proof that
    // the spawned task exited is the wiremock `expect(1)` on GET /mcp — if it
    // were still looping we'd see ≥2 hits.

    // Now make a POST. The "header_exists Mcp-Session-Id" mock above expects 0 hits;
    // if the cache was not cleared, this POST would carry the stale header and
    // route to that mock instead, failing the expect(0).
    let _ = transport.send_request(ping_request()).await;

    // wiremock verifies the expect(0) and expect(1) on Drop.
}

/// 3. Transient 5xx: listener does not exit on first 503.
#[tokio::test]
async fn test_sse_get_5xx_does_not_terminate_listener() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/mcp"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&mock_server)
        .await;

    let config = ConnectionConfig::default();
    let transport =
        HttpTransport::with_config(&format!("{}/mcp", mock_server.uri()), &config).unwrap();
    transport.connect().await.unwrap();
    transport.set_session_id("alive-session".to_string());

    let mut rx = transport.start_event_listener().await.unwrap();

    // First event: an Error("HTTP 503") — listener must NOT have exited.
    let evt = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("listener must emit an event within 2s")
        .expect("channel must yield Some(_)");
    match evt {
        ServerEvent::Error(msg) => {
            assert!(
                msg.contains("503"),
                "error must mention HTTP 503, got: {msg}"
            );
            assert!(
                !msg.to_lowercase().contains("listener exiting"),
                "5xx must NOT be terminal, got: {msg}"
            );
        }
        other => panic!("expected ServerEvent::Error, got {other:?}"),
    }

    // Bounded-window check: the spawned task must not close immediately after
    // a 5xx (it's mid-backoff, not exited). A timeout here is the success case;
    // the only failure mode is `Ok(None)` (channel closed because all senders
    // dropped — i.e. the spawned task returned). Note the transport keeps an
    // internal sender clone via `event_sender`, so `Ok(None)` here would only
    // happen if the transport itself dropped the listener early — which is
    // exactly the regression we want to catch.
    let immediate_close = tokio::time::timeout(Duration::from_millis(200), rx.recv()).await;
    assert!(
        !matches!(immediate_close, Ok(None)),
        "5xx must not close the listener within the backoff window, got {immediate_close:?}"
    );
}

/// 5. Compare-and-swap on cache clear: if a fresher session ID landed in the
/// cache while the GET was in flight, the 4xx terminal handler must NOT clobber
/// it. This races `client.connect()` writing the just-initialized session into
/// the cache against the spawned listener's in-flight GET completing with 4xx.
#[tokio::test]
async fn test_sse_4xx_does_not_clobber_fresher_session_id() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/mcp"))
        .respond_with(ResponseTemplate::new(400))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = ConnectionConfig::default();
    let transport =
        HttpTransport::with_config(&format!("{}/mcp", mock_server.uri()), &config).unwrap();
    transport.connect().await.unwrap();

    // Simulate the listener-built-GET-with-stale-snapshot case: pre-populate
    // session_id = "old", spawn the listener (which snapshots "old" before
    // sending), then before the 4xx response is processed, overwrite the cache
    // with "new" — mimicking initialize_session() landing mid-flight.
    transport.set_session_id("old".to_string());
    let mut rx = transport.start_event_listener().await.unwrap();

    // Race window: overwrite the cache with the fresher value before the
    // listener gets a chance to process the 400 response. Yield first so the
    // spawned task gets to run send().await.
    tokio::task::yield_now().await;
    transport.set_session_id("new".to_string());

    // Wait for the listener's terminal Error.
    let _ = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("listener must emit an event")
        .expect("channel must yield Some(_)");

    // The cache must still hold "new" — the 4xx handler must not clobber a
    // value that doesn't match what it sent. We assert behaviorally via the
    // next POST: install a header-matcher mock that requires Mcp-Session-Id=new.
    Mock::given(method("POST"))
        .and(path("/mcp"))
        .and(wiremock::matchers::header("Mcp-Session-Id", "new"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(json_rpc_ok()),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let _ = transport.send_request(ping_request()).await;
    // wiremock's expect(1) on the header("Mcp-Session-Id", "new") matcher
    // verifies the cache was NOT clobbered. If it had been cleared to None,
    // the POST would arrive without the header and the mock would not match.
}

/// 4. Stateless mode regression guard.
#[tokio::test]
async fn test_sse_listener_works_without_session_id() {
    let mock_server = MockServer::start().await;

    // SSE stream that delivers one notification and then closes.
    let sse_body =
        "event: message\ndata: {\"jsonrpc\":\"2.0\",\"method\":\"notifications/ping\"}\n\n";
    Mock::given(method("GET"))
        .and(path("/mcp"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "text/event-stream")
                .set_body_string(sse_body),
        )
        .expect(1..)
        .mount(&mock_server)
        .await;

    let config = ConnectionConfig::default();
    let transport =
        HttpTransport::with_config(&format!("{}/mcp", mock_server.uri()), &config).unwrap();
    transport.connect().await.unwrap();
    // NB: no set_session_id — simulates stateless server (spec-valid per
    // client.rs:337 "Server did not provide Mcp-Session-Id — stateless session").

    let mut rx = transport.start_event_listener().await.unwrap();

    // Must receive the notification — listener must NOT abort just because
    // session_id is None.
    let evt = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("listener must emit an event within 2s")
        .expect("channel must yield Some(_)");
    match evt {
        ServerEvent::Notification(json) => {
            assert_eq!(
                json.get("method").and_then(|v| v.as_str()),
                Some("notifications/ping")
            );
        }
        other => panic!("expected ServerEvent::Notification, got {other:?}"),
    }
}
