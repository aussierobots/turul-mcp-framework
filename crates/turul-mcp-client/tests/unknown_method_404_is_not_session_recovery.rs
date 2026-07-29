#![cfg(any(feature = "client-bilingual", feature = "client-2026-07-28-only"))]
//! A 404 on the 2026-07-28 lane must reach the caller, not trigger a handshake.
//!
//! 2026-07-28 maps an unknown method to HTTP 404 with `-32601` (the server side
//! is pinned by `error_mapping_2026.rs::unknown_method_gets_http_404_with_method_not_found`).
//! The client's 404 branch predates that: it reads 404 as "session unknown" and
//! recovers by re-running `initialize` — a method this revision removed. So a
//! peer that simply does not implement one method would be answered with a
//! handshake it cannot serve.
//!
//! Found by pointing `interop-client-probe` at a FastMCP server, which
//! implements no `completion/complete`. Nothing in the unit suite could see it:
//! it needs a peer whose capability set differs from our own server's.

use turul_mcp_client::transport::HttpTransport;
use turul_mcp_client::{McpClient, McpVersion};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Body of a `server/discover` reply, enough for the client to lock 2026-07-28.
fn discover_result() -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0", "id": 1,
        "result": {
            "resultType": "complete",
            "supportedVersions": ["2026-07-28"],
            "capabilities": {},
            "ttlMs": 0,
            "cacheScope": "public",
            "_meta": {
                "io.modelcontextprotocol/serverInfo": { "name": "peer", "version": "1.0.0" }
            }
        }
    })
}

#[tokio::test]
async fn a_404_unknown_method_is_returned_not_recovered_with_initialize() {
    let server = MockServer::start().await;

    // Discovery succeeds; every later POST is an unknown method.
    Mock::given(method("POST"))
        .and(path("/mcp"))
        .respond_with(ResponseTemplate::new(200).set_body_json(discover_result()))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/mcp"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "jsonrpc": "2.0", "id": 2,
            "error": { "code": -32601, "message": "Method not found" }
        })))
        .mount(&server)
        .await;

    let url = format!("{}/mcp", server.uri());
    let client = McpClient::new(Box::new(HttpTransport::new(&url).unwrap()), Default::default());
    client.connect().await.expect("discover");
    assert_eq!(client.negotiated_version().await, Some(McpVersion::V2026_07_28));

    let err = client
        .list_tools()
        .await
        .expect_err("an unknown method must surface as an error");

    // The point of the test: what went on the wire afterwards.
    let sent: Vec<String> = server
        .received_requests()
        .await
        .expect("recorded requests")
        .iter()
        .filter_map(|r| serde_json::from_slice::<serde_json::Value>(&r.body).ok())
        .filter_map(|v| v["method"].as_str().map(str::to_owned))
        .collect();

    assert!(
        !sent.iter().any(|m| m == "initialize" || m == "notifications/initialized"),
        "a 404 must not be recovered with a handshake this revision removed; sent {sent:?} \
         (error surfaced was: {err})"
    );
}
