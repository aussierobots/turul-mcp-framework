//! Regression test: `ConnectionConfig.max_redirects` is honored.
//!
//! Before 0.3.35, `HttpTransport::with_config` ignored `max_redirects` entirely;
//! reqwest fell back to its default redirect policy (10 hops). Clients that set
//! `max_redirects = 0` expecting to see raw 3xx responses silently got auto-followed
//! redirects instead.

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use turul_mcp_client::config::ConnectionConfig;
use turul_mcp_client::transport::Transport;
use turul_mcp_client::transport::http::HttpTransport;

/// With `max_redirects = 0, follow_redirects = true`, a 302 must NOT be followed.
/// The transport should surface the 3xx as an HTTP error (reqwest treats
/// exceeded-redirect as an error) rather than chasing the Location header.
#[tokio::test]
async fn max_redirects_zero_does_not_follow_302() {
    let mock_server = MockServer::start().await;

    // POST /mcp → 302 redirect to /elsewhere
    Mock::given(method("POST"))
        .and(path("/mcp"))
        .respond_with(
            ResponseTemplate::new(302).insert_header("Location", "/elsewhere"),
        )
        .expect(1) // Must be called exactly once — no auto-follow
        .mount(&mock_server)
        .await;

    // POST /elsewhere → 200 with a success body. If max_redirects were ignored,
    // reqwest's default policy would follow the 302 here and we'd see this mock hit.
    Mock::given(method("POST"))
        .and(path("/elsewhere"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0", "id": "req_0", "result": {}
                })),
        )
        .expect(0) // Must NOT be called — redirect should not be followed
        .mount(&mock_server)
        .await;

    let config = ConnectionConfig {
        follow_redirects: true,
        max_redirects: 0,
        ..Default::default()
    };

    let endpoint = format!("{}/mcp", mock_server.uri());
    let transport = HttpTransport::with_config(&endpoint, &config).unwrap();
    transport.connect().await.unwrap();

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "req_0",
        "method": "ping",
    });

    // Expect an error (reqwest surfaces "too many redirects" when limit is exceeded
    // on the very first hop). The exact error variant is not the point — the point
    // is that the mock at /elsewhere is never hit. wiremock's `.expect(0)` / `.expect(1)`
    // assertions on drop verify the hop count.
    let _ = transport.send_request(request).await;

    // wiremock verifies the expectations when `mock_server` drops, but surfacing any
    // mismatch explicitly gives a better failure message.
    mock_server.verify().await;
}
