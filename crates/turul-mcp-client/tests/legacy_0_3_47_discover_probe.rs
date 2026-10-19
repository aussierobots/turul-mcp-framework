#![cfg(feature = "client-bilingual")]
//! A turul-mcp-server 0.3.47 server answers the pre-initialize `server/discover`
//! probe with HTTP 400 + JSON-RPC `-32001 "Missing Mcp-Session-Id header"`.
//! The bilingual client must read that as a stateful 2025-11-25 server and
//! fall back; every other `-32001` must still abort negotiation.

use std::net::SocketAddr;
use turul_mcp_client::config::ClientConfig;
use turul_mcp_client::error::{McpClientError, ProtocolError};
use turul_mcp_client::transport::http::HttpTransport;
use turul_mcp_client::{McpClient, McpVersion};
use wiremock::matchers::{body_partial_json, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn start_legacy_server() -> SocketAddr {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let server = turul_mcp_server_legacy::McpServer::builder()
        .name("legacy")
        .version("0.3.47")
        .bind_address(addr)
        .sse(false)
        .build()
        .unwrap();
    tokio::spawn(async move { server.run().await });
    for _ in 0..50 {
        if tokio::net::TcpStream::connect(addr).await.is_ok() {
            return addr;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("0.3.47 server did not start");
}

fn client_for(url: &str) -> McpClient {
    let transport = Box::new(HttpTransport::new(url).unwrap());
    McpClient::new(transport, ClientConfig::default())
}

/// The exact reply of a 0.3.47 server, captured from the in-process server.
#[tokio::test]
async fn legacy_0_3_47_server_negotiates_2025_11_25() {
    let addr = start_legacy_server().await;
    let client = client_for(&format!("http://{addr}/mcp"));
    client
        .connect()
        .await
        .expect("a 0.3.47 server must connect via fallback");
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2025_11_25)
    );
}

/// Same status line and body a 0.3.47 server sent when captured:
/// `HTTP 400`, `{"jsonrpc":"2.0","id":"req_0","error":{"code":-32001,
/// "message":"Missing Mcp-Session-Id header. Call initialize first."}}`.
#[tokio::test]
async fn other_32001_still_aborts_negotiation() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(body_partial_json(
            serde_json::json!({"method": "server/discover"}),
        ))
        .respond_with(
            ResponseTemplate::new(400)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": "req_0",
                    "error": {"code": -32001, "message": "Session expired"}
                })),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = client_for(&format!("{}/mcp", server.uri()));
    let err = client.connect().await.expect_err("must abort");
    assert!(
        matches!(
            err,
            McpClientError::Protocol(ProtocolError::NegotiationFailed(_))
        ),
        "got {err:?}"
    );
}
