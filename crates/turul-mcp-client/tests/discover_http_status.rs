#![cfg(any(feature = "client-bilingual", feature = "client-2026-07-28-only"))]
//! An HTTP failure on the `server/discover` probe reaches the caller as that
//! HTTP status and its body: it is a
//! transport or authorization failure, not a protocol-version signal.
//!
//! Runs on the bilingual build and on `client-2026-07-28-only`. A server that
//! refuses the caller's origin answers 403 before reading the request, and
//! that must not read as "not a 2026-07-28 server".

use turul_mcp_client::error::TransportError;
use turul_mcp_client::transport::HttpTransport;
use turul_mcp_client::{ClientConfig, McpClient, McpClientError};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn connect_against(status: u16, body: &str) -> (McpClient, McpClientError) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(status).set_body_string(body))
        .mount(&server)
        .await;
    let url = format!("{}/mcp", server.uri());
    let client = McpClient::new(
        Box::new(HttpTransport::new(&url).unwrap()),
        ClientConfig::default(),
    );
    let err = client
        .connect()
        .await
        .expect_err("an HTTP failure aborts connect");
    (client, err)
}

fn assert_http_status(err: &McpClientError, expected: u16, body: &str) {
    match err {
        McpClientError::Transport(TransportError::HttpStatus { status, message }) => {
            assert_eq!(*status, expected, "status");
            assert!(message.contains(body), "body kept: {message:?}");
        }
        other => panic!("expected HttpStatus {expected}, got {other:?}"),
    }
}

#[tokio::test]
async fn probe_403_surfaces_status_and_body() {
    let (client, err) = connect_against(403, "Forbidden: Origin not allowed").await;
    assert_http_status(&err, 403, "Forbidden: Origin not allowed");
    assert_eq!(client.negotiated_version().await, None, "nothing is locked");
}

#[tokio::test]
async fn probe_401_surfaces_status_and_body() {
    let (_, err) = connect_against(401, "missing bearer token").await;
    assert_http_status(&err, 401, "missing bearer token");
}

/// Without `allow_legacy_gateway_fallback`, a 404 is a deployment problem, not
/// a version signal, on both builds.
#[tokio::test]
async fn probe_404_surfaces_status_and_body() {
    let (_, err) = connect_against(404, "no route").await;
    assert_http_status(&err, 404, "no route");
}

/// A 400 whose body is not a JSON-RPC error is an HTTP failure too.
#[tokio::test]
async fn probe_400_without_jsonrpc_body_surfaces_status_and_body() {
    let (_, err) = connect_against(400, "bad request").await;
    assert_http_status(&err, 400, "bad request");
}
