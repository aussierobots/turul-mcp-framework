//! Wire-level acceptance: "Clients that support sampling MUST declare the
//! sampling capability in `_meta.io.modelcontextprotocol/clientCapabilities`
//! on each request" (client/sampling).
//!
//! Each mock below only answers if the POST body's `_meta.clientCapabilities`
//! carries the `sampling` object — so a successful `tools/list` AND a
//! successful `tools/call` on the SAME connection prove the declaration rides
//! every request, not just the first. The negative test proves the field is
//! genuinely absent when the client never declared it.

use turul_mcp_client::config::{ClientConfig, DeclaredCapabilities};
use turul_mcp_client::transport::http::HttpTransport;
use turul_mcp_client::{McpClient, McpVersion};
use wiremock::matchers::{body_partial_json, header, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn start_2026_server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(header("Mcp-Method", "server/discover"))
        .and(header("MCP-Protocol-Version", "2026-07-28"))
        .and(body_partial_json(
            serde_json::json!({"method": "server/discover"}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0", "id": "d",
                    "result": {
                        "resultType": "complete", "ttlMs": 0, "cacheScope": "public",
                        "supportedVersions": ["2026-07-28"], "capabilities": {},
                        "serverInfo": { "name": "mock-2026", "version": "1.0.0" }
                    }
                })),
        )
        .mount(&server)
        .await;
    server
}

/// Mounts a response for `rpc_method` that ONLY matches if the request body's
/// `_meta.io.modelcontextprotocol/clientCapabilities.sampling` is present —
/// i.e. a successful call proves the declaration rode on THIS request.
async fn mount_sampling_gated_result(
    server: &MockServer,
    rpc_method: &str,
    result: serde_json::Value,
) {
    Mock::given(method("POST"))
        .and(header("Mcp-Method", rpc_method))
        .and(body_partial_json(serde_json::json!({
            "method": rpc_method,
            "params": {
                "_meta": {
                    "io.modelcontextprotocol/clientCapabilities": { "sampling": {} }
                }
            }
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(
                    serde_json::json!({ "jsonrpc": "2.0", "id": "x", "result": result }),
                ),
        )
        .mount(server)
        .await;
}

fn tools_list_result() -> serde_json::Value {
    serde_json::json!({
        "resultType": "complete", "ttlMs": 0, "cacheScope": "public",
        "tools": [{
            "name": "echo",
            "inputSchema": { "type": "object", "properties": {} }
        }]
    })
}

fn tools_call_result() -> serde_json::Value {
    serde_json::json!({
        "resultType": "complete",
        "content": [{"type": "text", "text": "ok"}],
        "isError": false
    })
}

/// Positive case: a sampling-declaring client's `tools/list` AND `tools/call`
/// (two independent operational requests on one connection) both carry the
/// sampling capability — proving "on each request", not just the first.
#[tokio::test]
async fn sampling_capability_rides_every_request_when_declared() {
    let server = start_2026_server().await;
    mount_sampling_gated_result(&server, "tools/list", tools_list_result()).await;
    mount_sampling_gated_result(&server, "tools/call", tools_call_result()).await;

    let url = format!("{}/mcp", server.uri());
    let transport = Box::new(HttpTransport::new(&url).unwrap());
    let config = ClientConfig {
        declared_capabilities: DeclaredCapabilities {
            sampling: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let client = McpClient::new(transport, config);
    client.connect().await.expect("connect to 2026 server");
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2026_07_28)
    );

    client
        .list_tools()
        .await
        .expect("tools/list must carry the sampling capability in _meta");
    client
        .call_tool("echo", serde_json::json!({}))
        .await
        .expect("tools/call must ALSO carry the sampling capability in _meta");
}

/// Negative case: against the identical sampling-gated mocks, a client that
/// never declared sampling gets no matching stub (wiremock's unmatched
/// default is a bare 404) — proving the capability is genuinely absent from
/// the wire body when not declared, not just permissively ignored server-side.
#[tokio::test]
async fn sampling_capability_is_absent_when_not_declared() {
    let server = start_2026_server().await;
    mount_sampling_gated_result(&server, "tools/list", tools_list_result()).await;

    let url = format!("{}/mcp", server.uri());
    let transport = Box::new(HttpTransport::new(&url).unwrap());
    let client = McpClient::new(transport, ClientConfig::default());
    client.connect().await.expect("connect to 2026 server");

    let result = client.list_tools().await;
    assert!(
        result.is_err(),
        "tools/list must NOT match the sampling-gated stub when sampling was never declared: {result:?}"
    );
}
