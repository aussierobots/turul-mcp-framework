//! The client on wasm32 under wasm-bindgen (ADR-040), run in Node and in
//! headless Firefox against `examples/wasm_e2e_fixture.rs`. Two URLs are fixed
//! at compile time: `TURUL_WASM_E2E_URL` (the running fixture) and
//! `TURUL_WASM_E2E_DEAD_URL` (a port with nothing listening).
//! `scripts/ci-gates.sh wasm` starts the fixture, checks the dead port and
//! sets both.
//!
//! ```text
//! TURUL_WASM_E2E_URL=http://127.0.0.1:52940/mcp \
//! TURUL_WASM_E2E_DEAD_URL=http://127.0.0.1:52941/mcp \
//! CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
//! cargo test --target wasm32-unknown-unknown -p turul-mcp-client \
//!     --no-default-features --features http,client-2026-07-28-only --test wasm
//! ```
#![cfg(target_arch = "wasm32")]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine as _;
use serde_json::json;
use turul_mcp_client::config::{ClientConfig, ConnectionConfig, RetryConfig};
use turul_mcp_client::error::TransportError;
use turul_mcp_client::transport::http::HttpTransport;
use turul_mcp_client::transport::{TransportFactory, TransportType};
use turul_mcp_client::{McpClient, McpClientError, McpVersion, ResourceContent};
use wasm_bindgen_test::wasm_bindgen_test;

const BLOB_URI: &str = "file:///fixture/blob.bin";
const BLOB_SHA256: &str = "40aff2e9d2d8922e47afd4648e6967497158785fbd1da870e7110266bf944880";
const SLOW_URI: &str = "file:///fixture/slow.txt";

/// The fixture URL. Panics, so a run without the fixture fails instead of passing vacuously.
#[allow(clippy::option_env_unwrap)] // unset must fail the test, not skip it
fn fixture_url() -> &'static str {
    option_env!("TURUL_WASM_E2E_URL")
        .expect("TURUL_WASM_E2E_URL must name a running wasm_e2e_fixture at build time")
}

/// A port with nothing listening, not on the Fetch bad-port list.
#[allow(clippy::option_env_unwrap)] // unset must fail the test, not skip it
fn dead_url() -> &'static str {
    option_env!("TURUL_WASM_E2E_DEAD_URL")
        .expect("TURUL_WASM_E2E_DEAD_URL must name a closed port at build time")
}

async fn connected(config: ClientConfig) -> McpClient {
    connected_over(
        HttpTransport::new(fixture_url()).expect("transport"),
        config,
    )
    .await
}

async fn connected_over(transport: HttpTransport, config: ClientConfig) -> McpClient {
    let client = McpClient::new(Box::new(transport), config);
    client.connect().await.expect("connect to the fixture");
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2026_07_28)
    );
    client
}

/// The full request shape (headers, `_meta`) is accepted by a real 2026-07-28
/// server, and the blob and its `_meta` come back intact.
#[wasm_bindgen_test]
async fn reads_a_blob_resource_with_its_meta() {
    assert_blob_reads(&connected(ClientConfig::default()).await).await;
}

/// The `ConnectionConfig` path also passes a browser's CORS preflight: on
/// wasm32 no `User-Agent` is set, even though the default config names one.
#[wasm_bindgen_test]
async fn default_connection_config_reads_the_blob() {
    let transport =
        HttpTransport::with_config(fixture_url(), &ConnectionConfig::default()).expect("transport");
    assert_blob_reads(&connected_over(transport, ClientConfig::default()).await).await;
}

async fn assert_blob_reads(client: &McpClient) {
    let contents = client
        .read_resource(BLOB_URI)
        .await
        .expect("resources/read");
    assert_eq!(contents.len(), 1);
    let ResourceContent::Blob(blob) = &contents[0] else {
        panic!("expected a blob, got {:?}", contents[0]);
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&blob.blob)
        .expect("base64");
    assert_eq!(bytes, (0..=255).collect::<Vec<u8>>());
    let meta = blob.meta.as_ref().expect("blob _meta");
    assert_eq!(meta.get("sha256"), Some(&json!(BLOB_SHA256)));
}

/// A JSON-RPC error from the server arrives as `ServerError` with its code and message.
#[wasm_bindgen_test]
async fn unknown_resource_is_a_structured_server_error() {
    let client = connected(ClientConfig::default()).await;
    let err = client
        .read_resource("file:///fixture/missing")
        .await
        .expect_err("unknown resource");
    match err {
        McpClientError::ServerError { code, message, .. } => {
            assert_eq!(code, -32602, "2026-07-28 resource-not-found code");
            assert!(
                message.contains("file:///fixture/missing"),
                "message names the URI: {message}"
            );
        }
        other => panic!("expected ServerError, got {other:?}"),
    }
}

/// The request timeout fires on wasm32 (no tokio timer exists there).
#[wasm_bindgen_test]
async fn request_timeout_fires() {
    let mut config = ClientConfig::default();
    config.timeouts.request = Duration::from_millis(100);
    config.retry.max_attempts = 1;
    let client = connected(config).await;
    let err = client
        .read_resource(SLOW_URI)
        .await
        .expect_err("slow resource");
    assert!(matches!(err, McpClientError::Timeout), "got {err:?}");
}

/// The streaming POST reader runs as a spawned local task.
#[wasm_bindgen_test]
async fn call_tool_with_progress_streams_progress_then_result() {
    let client = connected(ClientConfig::default()).await;
    let progress = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&progress);
    let result = client
        .call_tool_with_progress("progress_worker", json!({}), json!("p1"), move |p| {
            sink.lock().unwrap().push(p)
        })
        .await
        .expect("tools/call with progress");
    assert_eq!(
        result.structured_content,
        Some(json!({ "output": "worked" }))
    );
    let progress = progress.lock().unwrap();
    assert_eq!(progress.len(), 1, "one progress notification: {progress:?}");
    assert_eq!(progress[0].get("progress"), Some(&json!(0.5)));
    assert_eq!(progress[0].get("total"), Some(&json!(1.0)));
    assert_eq!(progress[0].get("progressToken"), Some(&json!("p1")));
}

/// Dropping a connected client runs its cleanup task, and the executor keeps working.
#[wasm_bindgen_test]
async fn dropping_a_client_leaves_the_executor_working() {
    let client = connected(ClientConfig::default()).await;
    drop(client);
    gloo_timers::future::sleep(Duration::from_millis(20)).await;
    assert_blob_reads(&connected(ClientConfig::default()).await).await;
}

/// A refused connection is a transport error, not a panic.
#[wasm_bindgen_test]
async fn unreachable_endpoint_is_a_transport_error() {
    let transport = HttpTransport::new(dead_url()).expect("transport");
    let client = McpClient::new(Box::new(transport), ClientConfig::default());
    let err = client.connect().await.expect_err("nothing listens there");
    assert!(
        matches!(err, McpClientError::Transport(TransportError::Http(_))),
        "got {err:?}"
    );
}

/// fetch always follows redirects, so `follow_redirects: false` is refused.
#[wasm_bindgen_test]
fn follow_redirects_false_is_refused() {
    let config = ConnectionConfig {
        follow_redirects: false,
        ..ConnectionConfig::default()
    };
    let err = HttpTransport::with_config("http://127.0.0.1:9/mcp", &config)
        .map(|_| ())
        .expect_err("refused");
    match err {
        McpClientError::Transport(TransportError::ConnectionFailed(msg)) => {
            assert!(msg.contains("redirect"), "message: {msg}")
        }
        other => panic!("expected ConnectionFailed, got {other:?}"),
    }
}

/// Retry jitter draws from the JavaScript host's random source.
#[wasm_bindgen_test]
fn retry_jitter_stays_in_range() {
    let retry = RetryConfig {
        initial_delay: Duration::from_millis(1000),
        max_delay: Duration::from_secs(10),
        jitter: 0.5,
        exponential_backoff: false,
        ..RetryConfig::default()
    };
    let delays: Vec<Duration> = (0..32).map(|_| retry.delay_for_attempt(2)).collect();
    for delay in &delays {
        assert!(
            (Duration::from_millis(1000)..=Duration::from_millis(1500)).contains(delay),
            "delay {delay:?}"
        );
    }
    assert!(
        delays.iter().any(|d| *d != delays[0]),
        "32 draws were identical: {delays:?}"
    );
}

/// `SseTransport` is not compiled on wasm32; the factory offers HTTP only (ADR-040 §3).
#[wasm_bindgen_test]
fn transport_factory_offers_http_only() {
    assert_eq!(
        TransportFactory::available_transports(),
        vec![TransportType::Http]
    );
    let err = TransportFactory::from_url("http://127.0.0.1:52940/sse")
        .map(|_| ())
        .expect_err("an SSE URL is refused");
    assert!(
        matches!(
            err,
            McpClientError::Transport(TransportError::ConnectionFailed(_))
        ),
        "got {err:?}"
    );
    let err = TransportFactory::create(TransportType::Sse, "http://127.0.0.1:52940/mcp")
        .map(|_| ())
        .expect_err("the SSE type is refused");
    assert!(
        matches!(
            err,
            McpClientError::Transport(TransportError::ConnectionFailed(_))
        ),
        "got {err:?}"
    );
}
