//! Wire-layer compliance tests for the 2026-07-28 Lambda lane.
//!
//! Drives the production entry `LambdaMcpHandler::handle_streaming()` (the
//! Lambda runtime's real dispatch path for streaming responses) and asserts
//! on HTTP status + JSON-RPC error body — not on framework-internal types.
#![cfg(feature = "protocol-2026-07-28")]

use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{BodyExt, combinators::UnsyncBoxBody};
use lambda_http::Body as LambdaBody;
use serde_json::json;

use turul_mcp_aws_lambda::{LambdaMcpHandler, LambdaMcpServerBuilder};
use turul_mcp_derive::McpTool;
use turul_mcp_protocol::MCP_VERSION;
use turul_mcp_session_storage::InMemorySessionStorage;

#[derive(McpTool, Clone, Default)]
#[tool(name = "ping_tool", description = "Return pong", output = String)]
struct PingTool {}

impl PingTool {
    async fn execute(
        &self,
        _session: Option<turul_mcp_server::SessionContext>,
    ) -> turul_mcp_server::McpResult<String> {
        Ok("pong".to_string())
    }
}

async fn build_handler() -> LambdaMcpHandler {
    let server = LambdaMcpServerBuilder::new()
        .name("wire-compliance-test")
        .version("1.0.0")
        .tool(PingTool::default())
        .storage(Arc::new(InMemorySessionStorage::new()))
        .sse(false)
        .build()
        .await
        .expect("build lambda server");
    server.handler().await.expect("create lambda handler")
}

fn request_meta() -> serde_json::Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": MCP_VERSION,
        "io.modelcontextprotocol/clientInfo": { "name": "test-client", "version": "1.0.0" },
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

/// Builds a POST request with the SEP-2243 modern headers (MCP-Protocol-Version
/// matching the body's `_meta` protocolVersion, Mcp-Method matching the body
/// method) that Streamable HTTP's Server Validation requires on this lane.
fn modern_request(method: &str, params: serde_json::Value) -> lambda_http::Request {
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    });
    http::Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", MCP_VERSION)
        .header("Mcp-Method", method)
        .body(LambdaBody::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}

async fn collect(
    response: lambda_http::Response<UnsyncBoxBody<Bytes, hyper::Error>>,
) -> (http::StatusCode, serde_json::Value) {
    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    assert_eq!(
        content_type.as_deref(),
        Some("application/json"),
        "wire responses on this lane are JSON, not SSE"
    );
    let bytes = response
        .into_body()
        .collect()
        .await
        .map(|c| c.to_bytes())
        .unwrap_or_default();
    let json = serde_json::from_slice(&bytes)
        .unwrap_or_else(|e| panic!("response body must be JSON: {e}"));
    (status, json)
}

/// T1: a modern `server/discover` request succeeds and names the versions
/// this build supports.
#[tokio::test]
async fn discover_returns_supported_versions() {
    let handler = build_handler().await;
    let request = modern_request("server/discover", json!({ "_meta": request_meta() }));

    let response = handler
        .handle_streaming(request)
        .await
        .expect("handle_streaming");
    let (status, body) = collect(response).await;

    assert_eq!(status, 200, "server/discover must succeed: {body}");
    let supported = body["result"]["supportedVersions"]
        .as_array()
        .unwrap_or_else(|| panic!("missing result.supportedVersions: {body}"));
    assert!(
        supported.iter().any(|v| v == MCP_VERSION),
        "supportedVersions must name {MCP_VERSION}: {body}"
    );
}

/// T2: a request whose MCP-Protocol-Version header names a version this
/// build does not implement is rejected with the recognized modern error
/// (UnsupportedProtocolVersionError, -32022) rather than a generic failure —
/// so a modern client retries with a supported version instead of falling
/// back to a legacy handshake.
#[tokio::test]
async fn unsupported_protocol_version_returns_recognized_modern_error() {
    let handler = build_handler().await;
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "server/discover",
        "params": { "_meta": request_meta() },
    });
    let request = http::Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "1900-01-01")
        .header("Mcp-Method", "server/discover")
        .body(LambdaBody::from(serde_json::to_string(&body).unwrap()))
        .unwrap();

    let response = handler
        .handle_streaming(request)
        .await
        .expect("handle_streaming");
    let (status, body) = collect(response).await;

    assert_eq!(status, 400, "unsupported version must be HTTP 400: {body}");
    assert_eq!(
        body["error"]["code"].as_i64(),
        Some(-32022),
        "expected UnsupportedProtocolVersionError: {body}"
    );
    assert!(
        body["error"]["data"]["supported"].is_array(),
        "error.data.supported must list this build's versions: {body}"
    );
}

/// T3: a true legacy client — no MCP-Protocol-Version header at all — sending
/// `initialize` gets HeaderMismatch (-32020) naming the versions this build
/// supports. Per the Versioning page's Backward Compatibility section, this
/// may be the client's only diagnostic since it has no fall-forward path.
#[tokio::test]
async fn initialize_with_no_modern_headers_names_supported_versions() {
    let handler = build_handler().await;
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": { "name": "legacy-client", "version": "1.0.0" }
        },
    });
    let request = http::Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .body(LambdaBody::from(serde_json::to_string(&body).unwrap()))
        .unwrap();

    let response = handler
        .handle_streaming(request)
        .await
        .expect("handle_streaming");
    let (status, body) = collect(response).await;

    assert_eq!(status, 400, "missing header must be HTTP 400: {body}");
    assert_eq!(
        body["error"]["code"].as_i64(),
        Some(turul_mcp_protocol::headers::ERROR_CODE_HEADER_MISMATCH),
        "expected HeaderMismatch: {body}"
    );
    assert_eq!(
        body["error"]["data"]["supported"],
        json!([MCP_VERSION]),
        "the initialize rejection must name the supported versions: {body}"
    );
}

/// T4: `notifications/cancelled` is the framework's documented accept-and-
/// ignore contract (parity with the non-Lambda builder) — HTTP 202, no error
/// body. This is not a transport MUST: Streamable HTTP's own prose expects
/// no cancelled notification at all on this transport (closing the request's
/// response stream is the cancellation signal); this test only asserts the
/// framework does not reject a client that sends one anyway.
#[tokio::test]
async fn notifications_cancelled_is_accepted_and_ignored() {
    let handler = build_handler().await;
    let body = json!({
        "jsonrpc": "2.0",
        "method": "notifications/cancelled",
        "params": { "requestId": 99, "reason": "client gave up" },
    });
    let request = http::Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", MCP_VERSION)
        .header("Mcp-Method", "notifications/cancelled")
        .body(LambdaBody::from(serde_json::to_string(&body).unwrap()))
        .unwrap();

    let response = handler
        .handle_streaming(request)
        .await
        .expect("handle_streaming");
    assert_eq!(
        response.status(),
        202,
        "notification POSTs return 202 regardless of dispatch outcome"
    );
}

/// T5: methods absent from the 2026 schema (`ping`, `roots/list`, and
/// `initialize` — its handshake is not registered on this lane) all answer
/// HTTP 404 + JSON-RPC -32601, matching the transport contract for a method
/// this server does not implement. `initialize` additionally carries
/// `error.data.supported`: the shared 404 branch special-cases it so a
/// client sending modern headers with a legacy method still gets the
/// version diagnostic (the Versioning page's Backward Compatibility SHOULD).
#[tokio::test]
async fn unregistered_methods_return_404_method_not_found() {
    let handler = build_handler().await;

    for method in ["ping", "roots/list", "initialize"] {
        let request = modern_request(method, json!({ "_meta": request_meta() }));
        let response = handler
            .handle_streaming(request)
            .await
            .unwrap_or_else(|e| panic!("handle_streaming for '{method}': {e}"));
        let (status, body) = collect(response).await;

        assert_eq!(status, 404, "'{method}' must be HTTP 404: {body}");
        assert_eq!(
            body["error"]["code"].as_i64(),
            Some(-32601),
            "'{method}' must be -32601 method-not-found: {body}"
        );
    }

    // initialize additionally names the supported versions (see doc comment above).
    let request = modern_request("initialize", json!({ "_meta": request_meta() }));
    let response = handler
        .handle_streaming(request)
        .await
        .expect("handle_streaming for 'initialize'");
    let (_, body) = collect(response).await;
    assert_eq!(
        body["error"]["data"]["supported"],
        json!([MCP_VERSION]),
        "initialize's -32601 must name exactly this build's supported versions: {body}"
    );
}
