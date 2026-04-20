//! Regression test: concurrent requests through one transport run in parallel,
//! not serialized behind an outer Mutex.
//!
//! Before the `&self` refactor, `McpClient::transport` was wrapped in a
//! `tokio::sync::Mutex<BoxedTransport>` and every `send_request` acquired
//! that lock for the entire round-trip. N parallel `call_tool` calls
//! executed sequentially, totalling N × per-call latency.
//!
//! After the refactor, `send_request(&self, …)` lets multiple calls share an
//! `Arc<BoxedTransport>` without any client-level Mutex — `reqwest::Client`
//! already handles concurrent HTTP POSTs via its internal connection pool.

use std::sync::Arc;
use std::time::{Duration, Instant};

use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

use turul_mcp_client::transport::Transport;
use turul_mcp_client::transport::http::HttpTransport;

/// Each mock response takes this long. With N=10 concurrent requests and
/// no serialization, wall time ≈ DELAY + overhead. With serialization, it
/// would be N × DELAY.
const PER_CALL_DELAY: Duration = Duration::from_millis(150);
const PARALLELISM: usize = 10;

#[tokio::test]
async fn test_concurrent_send_request_runs_in_parallel() {
    let mock_server = MockServer::start().await;

    // All POSTs return after PER_CALL_DELAY. wiremock can serve matched
    // mocks concurrently, so the server itself is not the bottleneck.
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0", "id": "req_0", "result": {}
                }))
                .set_delay(PER_CALL_DELAY),
        )
        .mount(&mock_server)
        .await;

    let transport =
        Arc::new(HttpTransport::new(&format!("{}/mcp", mock_server.uri())).unwrap());
    transport.connect().await.unwrap();

    let start = Instant::now();

    let mut handles = Vec::with_capacity(PARALLELISM);
    for i in 0..PARALLELISM {
        let t = Arc::clone(&transport);
        handles.push(tokio::spawn(async move {
            let req = serde_json::json!({
                "jsonrpc": "2.0",
                "id": format!("req_{}", i),
                "method": "ping",
                "params": {}
            });
            t.send_request(req).await
        }));
    }

    for h in handles {
        h.await
            .expect("task panicked")
            .expect("request failed");
    }

    let elapsed = start.elapsed();

    // If requests were serialized, wall time would be ≥ N * DELAY = 1500ms.
    // If parallel, it should be close to DELAY + per-request overhead.
    // Assert the wall time is less than half of the serial path — a
    // generous threshold that still fails if the transport Mutex returns.
    let serial_worst_case = PER_CALL_DELAY * PARALLELISM as u32;
    let threshold = serial_worst_case / 2;

    assert!(
        elapsed < threshold,
        "Expected concurrent requests to finish in < {:?} (half of serial \
         worst case {:?}). Actual wall time: {:?}. This suggests requests \
         are being serialized somewhere in the transport stack.",
        threshold,
        serial_worst_case,
        elapsed
    );

    // Also assert we beat the serial path by a meaningful margin.
    assert!(
        elapsed < PER_CALL_DELAY * 3,
        "Expected wall time close to single-call latency. Actual: {:?}. \
         Single-call latency: {:?}.",
        elapsed,
        PER_CALL_DELAY
    );
}
