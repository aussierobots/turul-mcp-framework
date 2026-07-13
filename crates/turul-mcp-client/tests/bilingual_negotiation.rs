//! Acceptance tests for per-connection wire-version negotiation.
//!
//! One bilingual `McpClient` (the default build) connects to three different
//! wiremock servers and locks the correct wire spec for each: 2026-07-28 when
//! the server answers `server/discover`, 2025-11-25 when it returns `-32601`
//! (no discover), and aborts WITHOUT downgrade when the probe gets an HTTP 4xx.
//! This is the executable form of the "one client speaks both specs" minimum.

use turul_mcp_client::config::ClientConfig;
use turul_mcp_client::transport::http::HttpTransport;
use turul_mcp_client::{McpClient, McpVersion};
use wiremock::matchers::{body_partial_json, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// The SSE GET listener fires on connect; return 404 so it exits cleanly
/// (v0.3.38 4xx-terminal contract) without disrupting the test.
async fn mount_sse_404(server: &MockServer) {
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .mount(server)
        .await;
}

async fn connect_client(server: &MockServer) -> (McpClient, turul_mcp_client::McpClientResult<()>) {
    let url = format!("{}/mcp", server.uri());
    let transport = Box::new(HttpTransport::new(&url).unwrap());
    let client = McpClient::new(transport, ClientConfig::default());
    let result = client.connect().await;
    (client, result)
}

#[tokio::test]
async fn bilingual_client_locks_2026_when_server_answers_discover() {
    let server = MockServer::start().await;
    mount_sse_404(&server).await;

    // server/discover returns a valid DiscoverResult => this is a 2026 server.
    Mock::given(method("POST"))
        .and(body_partial_json(
            serde_json::json!({"method": "server/discover"}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": "req_0",
                    "result": {
                        "resultType": "complete",
                        "ttlMs": 0,
                        "cacheScope": "public",
                        "supportedVersions": ["2026-07-28"],
                        "capabilities": {},
                        "serverInfo": { "name": "mock-2026", "version": "1.0.0" }
                    }
                })),
        )
        .mount(&server)
        .await;

    let (client, result) = connect_client(&server).await;
    result.expect("connect against a 2026 server must succeed");
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2026_07_28),
        "a server answering server/discover must lock the connection to 2026-07-28"
    );
}

#[tokio::test]
async fn bilingual_client_locks_2025_when_server_lacks_discover() {
    let server = MockServer::start().await;
    mount_sse_404(&server).await;

    // A real 2025-11-25 server has no server/discover => -32601 => fall back.
    Mock::given(method("POST"))
        .and(body_partial_json(
            serde_json::json!({"method": "server/discover"}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": "req_0",
                    "error": { "code": -32601, "message": "Method not found" }
                })),
        )
        .mount(&server)
        .await;

    // initialize: session ID + minimal valid InitializeResult.
    Mock::given(method("POST"))
        .and(body_partial_json(
            serde_json::json!({"method": "initialize"}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .insert_header("Mcp-Session-Id", "sess-bilingual")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": "req_0",
                    "result": {
                        "protocolVersion": "2025-11-25",
                        "capabilities": { "tools": { "listChanged": false } },
                        "serverInfo": { "name": "mock-2025", "version": "1.0.0" }
                    }
                })),
        )
        .mount(&server)
        .await;

    // notifications/initialized: 202 Accepted per MCP spec.
    Mock::given(method("POST"))
        .and(body_partial_json(
            serde_json::json!({"method": "notifications/initialized"}),
        ))
        .respond_with(ResponseTemplate::new(202))
        .mount(&server)
        .await;

    let (client, result) = connect_client(&server).await;
    result.expect("connect against a 2025 server must succeed via fallback");
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2025_11_25),
        "a -32601 on server/discover must fall back to and lock 2025-11-25"
    );
}

/// Versioning §Backward Compatibility: "a recognized modern JSON-RPC error
/// (such as UnsupportedProtocolVersionError) identifies a modern server: the
/// client retries with a supported version rather than falling back" — an
/// HTTP 400 whose body carries -32022 (the canonical code since the
/// 2026-07-02 error-code renumbering) with data.supported must fall back to
/// 2025-11-25 through the real probe path (not abort like a bare 4xx).
#[tokio::test]
async fn bilingual_client_falls_back_on_400_with_32022_body() {
    let server = MockServer::start().await;
    mount_sse_404(&server).await;

    // A validating modern-but-2025-only server: 400 + structured -32022.
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
                    "error": {
                        "code": -32022,
                        "message": "Unsupported protocol version",
                        "data": { "supported": ["2025-11-25"], "requested": "2026-07-28" }
                    }
                })),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(body_partial_json(
            serde_json::json!({"method": "initialize"}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .insert_header("Mcp-Session-Id", "sess-32022")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": "req_0",
                    "result": {
                        "protocolVersion": "2025-11-25",
                        "capabilities": { "tools": { "listChanged": false } },
                        "serverInfo": { "name": "mock-2025", "version": "1.0.0" }
                    }
                })),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(body_partial_json(
            serde_json::json!({"method": "notifications/initialized"}),
        ))
        .respond_with(ResponseTemplate::new(202))
        .mount(&server)
        .await;

    let (client, result) = connect_client(&server).await;
    result.expect("-32022 in a 400 body must trigger 2025 fallback, not abort");
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2025_11_25),
        "a structured -32022 must fall back to and lock 2025-11-25"
    );
}

/// Negative case: `-32004` was the pre-2026-07-02-renumbering
/// UnsupportedProtocolVersionError allocation. It is now implementation-defined
/// and unrelated to this client (no alias, no backward-compat recognition) —
/// a 400 body carrying it must be treated as an unrecognized JSON-RPC error
/// and abort the connect, not silently fall back to 2025-11-25.
#[tokio::test]
async fn bilingual_client_treats_pre_renumbering_32004_as_unrecognized_and_aborts() {
    let server = MockServer::start().await;
    mount_sse_404(&server).await;

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
                    "error": {
                        "code": -32004,
                        "message": "Unsupported protocol version",
                        "data": { "supported": ["2025-11-25"], "requested": "2026-07-28" }
                    }
                })),
        )
        .mount(&server)
        .await;

    let (client, result) = connect_client(&server).await;
    assert!(
        result.is_err(),
        "an unrecognized -32004 body must abort the connect, not fall back to 2025-11-25"
    );
    assert_eq!(
        client.negotiated_version().await,
        None,
        "no wire version may be locked when negotiation aborts"
    );
}

#[tokio::test]
async fn bilingual_client_aborts_on_4xx_without_downgrade() {
    let server = MockServer::start().await;
    mount_sse_404(&server).await;

    // HTTP 403 on the probe is an authorization failure, NOT a version signal.
    Mock::given(method("POST"))
        .and(body_partial_json(
            serde_json::json!({"method": "server/discover"}),
        ))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;

    let (client, result) = connect_client(&server).await;
    assert!(
        result.is_err(),
        "HTTP 403 on server/discover must abort the connect, not silently downgrade"
    );
    assert_eq!(
        client.negotiated_version().await,
        None,
        "no wire version may be locked when negotiation aborts"
    );
}

#[tokio::test]
async fn bilingual_client_round_trips_tools_list_against_2026_server() {
    let server = MockServer::start().await;
    mount_sse_404(&server).await;

    // server/discover => 2026 server; the connection locks to 2026-07-28.
    Mock::given(method("POST"))
        .and(body_partial_json(
            serde_json::json!({"method": "server/discover"}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": "req_0",
                    "result": {
                        "resultType": "complete",
                        "ttlMs": 0,
                        "cacheScope": "public",
                        "supportedVersions": ["2026-07-28"],
                        "capabilities": {},
                        "serverInfo": { "name": "mock-2026", "version": "1.0.0" }
                    }
                })),
        )
        .mount(&server)
        .await;

    // tools/list: the matcher REQUIRES the 2026 per-request `_meta` to be present,
    // so this only responds if the client sent a 2026-shaped request. The body is a
    // 2026 `ListToolsResult` (resultType + CacheableResult mixin + tools).
    Mock::given(method("POST"))
        .and(body_partial_json(serde_json::json!({
            "method": "tools/list",
            "params": { "_meta": { "io.modelcontextprotocol/protocolVersion": "2026-07-28" } }
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": "req_1",
                    "result": {
                        "resultType": "complete",
                        "ttlMs": 0,
                        "cacheScope": "public",
                        "tools": [{
                            "name": "echo",
                            "description": "Echo a message",
                            "inputSchema": { "type": "object", "properties": { "msg": { "type": "string" } } }
                        }]
                    }
                })),
        )
        .mount(&server)
        .await;

    let (client, result) = connect_client(&server).await;
    result.expect("connect against a 2026 server must succeed");
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2026_07_28)
    );

    // The round-trip: a 2026-shaped tools/list request that parses the 2026 result.
    let tools = client
        .list_tools()
        .await
        .expect("tools/list must round-trip against a 2026 server");
    assert_eq!(
        tools.len(),
        1,
        "expected the one tool the 2026 server returned"
    );
    assert_eq!(tools[0].name, "echo");
}

/// TX/GAP-7 (matrix: "FIXED 2026-06-11", silently regressed at the
/// 2026-07-02 error-code renumbering): SEP-2243 §Client Behavior — on a
/// HeaderMismatch rejection "the client SHOULD call tools/list to obtain the
/// current inputSchema, then retry the original request with the appropriate
/// headers." `call_tool` must recognize the current HeaderMismatch code
/// (`-32020`) and perform exactly one `tools/list` refresh + one retry.
#[tokio::test]
async fn call_tool_recovers_from_header_mismatch_with_one_refresh_and_retry() {
    let server = MockServer::start().await;
    mount_sse_404(&server).await;

    // server/discover => 2026 server; the connection locks to 2026-07-28.
    Mock::given(method("POST"))
        .and(body_partial_json(
            serde_json::json!({"method": "server/discover"}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": "req_0",
                    "result": {
                        "resultType": "complete",
                        "ttlMs": 0,
                        "cacheScope": "public",
                        "supportedVersions": ["2026-07-28"],
                        "capabilities": {},
                        "serverInfo": { "name": "mock-2026", "version": "1.0.0" }
                    }
                })),
        )
        .mount(&server)
        .await;

    // The refresh_tools() call after the mismatch. No x-mcp-header
    // annotations needed here — this test proves the retry fires, not the
    // SEP-2243 header-mirroring itself (covered by e2e_2026_real_server.rs).
    Mock::given(method("POST"))
        .and(body_partial_json(serde_json::json!({"method": "tools/list"})))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": "req_1",
                    "result": {
                        "resultType": "complete",
                        "ttlMs": 0,
                        "cacheScope": "public",
                        "tools": [{
                            "name": "echo",
                            "description": "Echo a message",
                            "inputSchema": { "type": "object", "properties": { "msg": { "type": "string" } } }
                        }]
                    }
                })),
        )
        .expect(1)
        .mount(&server)
        .await;

    // First tools/call: rejected with the current HeaderMismatch code.
    // Matched first (equal default priority, earlier mount wins) and
    // exhausts itself after one use, so the second call below falls through
    // to the success mock.
    Mock::given(method("POST"))
        .and(body_partial_json(serde_json::json!({"method": "tools/call"})))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": "req_2",
                    "error": {
                        "code": -32020,
                        "message": "Header mismatch: Mcp-Param-Msg header omitted but the parameter is present in the request body"
                    }
                })),
        )
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;

    // The retry after refresh_tools(): succeeds.
    Mock::given(method("POST"))
        .and(body_partial_json(serde_json::json!({"method": "tools/call"})))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": "req_3",
                    "result": {
                        "resultType": "complete",
                        "ttlMs": 0,
                        "cacheScope": "public",
                        "content": [{ "type": "text", "text": "echoed" }],
                        "isError": false
                    }
                })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let (client, result) = connect_client(&server).await;
    result.expect("connect against a 2026 server must succeed");

    let call_result = client
        .call_tool("echo", serde_json::json!({ "msg": "hi" }))
        .await
        .expect("the retry, after the tools/list refresh, must succeed");
    assert!(!call_result.is_error.unwrap_or(false));
    let text = serde_json::to_string(&call_result).unwrap_or_default();
    assert!(text.contains("echoed"), "{text}");

    // wiremock verifies exactly one tools/list refresh and exactly one retry
    // via the `.expect(1)` on each mock, checked when `server` drops.
}

/// Streamable HTTP servers answer protocol-level rejections with HTTP 400
/// and a JSON-RPC error body. The transport must surface that envelope to
/// the same JSON-RPC error classification a 200-body error takes — here the
/// HeaderMismatch refresh-retry — instead of burying it as a generic
/// transport failure. Same flow as the 200-body test above, but the
/// rejection arrives as a plain-JSON 400.
#[tokio::test]
async fn call_tool_recovers_from_plain_json_400_header_mismatch() {
    let server = MockServer::start().await;
    mount_sse_404(&server).await;

    Mock::given(method("POST"))
        .and(body_partial_json(
            serde_json::json!({"method": "server/discover"}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": "req_0",
                    "result": {
                        "resultType": "complete",
                        "ttlMs": 0,
                        "cacheScope": "public",
                        "supportedVersions": ["2026-07-28"],
                        "capabilities": {},
                        "serverInfo": { "name": "mock-2026", "version": "1.0.0" }
                    }
                })),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(body_partial_json(serde_json::json!({"method": "tools/list"})))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": "req_1",
                    "result": {
                        "resultType": "complete",
                        "ttlMs": 0,
                        "cacheScope": "public",
                        "tools": [{
                            "name": "echo",
                            "description": "Echo a message",
                            "inputSchema": { "type": "object", "properties": { "msg": { "type": "string" } } }
                        }]
                    }
                })),
        )
        .expect(1)
        .mount(&server)
        .await;

    // First tools/call: HTTP 400 whose body carries the HeaderMismatch
    // envelope — the shape a validating server uses when the caller does
    // not negotiate an SSE response.
    Mock::given(method("POST"))
        .and(body_partial_json(serde_json::json!({"method": "tools/call"})))
        .respond_with(
            ResponseTemplate::new(400)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": "req_2",
                    "error": {
                        "code": -32020,
                        "message": "Header mismatch: Mcp-Param-Msg header omitted but the parameter is present in the request body"
                    }
                })),
        )
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(body_partial_json(serde_json::json!({"method": "tools/call"})))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": "req_3",
                    "result": {
                        "resultType": "complete",
                        "ttlMs": 0,
                        "cacheScope": "public",
                        "content": [{ "type": "text", "text": "echoed" }],
                        "isError": false
                    }
                })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let (client, result) = connect_client(&server).await;
    result.expect("connect against a 2026 server must succeed");

    let call_result = client
        .call_tool("echo", serde_json::json!({ "msg": "hi" }))
        .await
        .expect("a plain-JSON 400 HeaderMismatch must reach the refresh-retry path and succeed");
    assert!(!call_result.is_error.unwrap_or(false));
}

/// Only status 400 is rescued into JSON-RPC error classification. A 404 —
/// even with a JSON body — must stay a transport-level HttpStatus error:
/// session-expiry recovery keys on it.
#[tokio::test]
async fn http_404_with_json_body_stays_a_transport_error() {
    let server = MockServer::start().await;
    mount_sse_404(&server).await;

    Mock::given(method("POST"))
        .and(body_partial_json(
            serde_json::json!({"method": "server/discover"}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": "req_0",
                    "result": {
                        "resultType": "complete",
                        "ttlMs": 0,
                        "cacheScope": "public",
                        "supportedVersions": ["2026-07-28"],
                        "capabilities": {},
                        "serverInfo": { "name": "mock-2026", "version": "1.0.0" }
                    }
                })),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(body_partial_json(serde_json::json!({"method": "tools/call"})))
        .respond_with(
            ResponseTemplate::new(404)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": "req_1",
                    "error": { "code": -32601, "message": "Method 'tools/call' not found" }
                })),
        )
        .mount(&server)
        .await;

    let (client, result) = connect_client(&server).await;
    result.expect("connect against a 2026 server must succeed");

    let err = client
        .call_tool("echo", serde_json::json!({ "msg": "hi" }))
        .await
        .expect_err("a 404 must surface as an error");
    assert!(
        matches!(
            &err,
            turul_mcp_client::McpClientError::Transport(
                turul_mcp_client::error::TransportError::HttpStatus { status: 404, .. }
            )
        ),
        "404 must remain a transport HttpStatus error, got: {err:?}"
    );
}
