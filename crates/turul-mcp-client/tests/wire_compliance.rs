//! Wire-level protocol compliance tests using wiremock.
//!
//! These tests verify that the HTTP transport sends the correct headers
//! on the wire. They use `HttpTransport` directly (not `McpClient`) to
//! avoid needing session initialization.

use wiremock::matchers::{header, headers, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

use turul_mcp_client::config::ConnectionConfig;
use turul_mcp_client::transport::Transport;
use turul_mcp_client::transport::http::HttpTransport;

fn json_rpc_ok() -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0", "id": "req_0", "result": {}
    })
}

fn ping_request() -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0", "id": "req_0", "method": "ping", "params": {}
    })
}

#[tokio::test]
async fn test_custom_headers_appear_on_outbound_requests() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(header("X-Custom", "test-value"))
        .and(header("Authorization", "Bearer tok"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(json_rpc_ok()),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let mut headers = std::collections::HashMap::new();
    headers.insert("X-Custom".to_string(), "test-value".to_string());
    headers.insert("Authorization".to_string(), "Bearer tok".to_string());

    let config = ConnectionConfig {
        headers: Some(headers),
        ..Default::default()
    };

    let transport =
        HttpTransport::with_config(&format!("{}/mcp", mock_server.uri()), &config).unwrap();
    transport.connect().await.unwrap();

    let _ = transport.send_request(ping_request()).await;
    // wiremock verifies header matchers via expect(1)
}

#[tokio::test]
async fn test_custom_user_agent() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(header("User-Agent", "my-app/2.0"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(json_rpc_ok()),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = ConnectionConfig {
        user_agent: Some("my-app/2.0".to_string()),
        ..Default::default()
    };

    let transport =
        HttpTransport::with_config(&format!("{}/mcp", mock_server.uri()), &config).unwrap();
    transport.connect().await.unwrap();

    let _ = transport.send_request(ping_request()).await;
}

#[tokio::test]
async fn test_no_redirects_policy() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(302).insert_header("Location", "http://example.com/redirect"),
        )
        .mount(&mock_server)
        .await;

    let config = ConnectionConfig {
        follow_redirects: false,
        ..Default::default()
    };

    let transport =
        HttpTransport::with_config(&format!("{}/mcp", mock_server.uri()), &config).unwrap();
    transport.connect().await.unwrap();

    let result = transport.send_request(ping_request()).await;
    assert!(
        result.is_err(),
        "302 should not be followed when redirects disabled"
    );
}

#[tokio::test]
async fn test_accept_header_on_post_requests() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(headers(
            "Accept",
            vec!["application/json", "text/event-stream"],
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(json_rpc_ok()),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let transport = HttpTransport::new(&format!("{}/mcp", mock_server.uri())).unwrap();
    transport.connect().await.unwrap();

    let _ = transport.send_request(ping_request()).await;
}

#[tokio::test]
async fn test_notification_post_includes_accept_header() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(headers(
            "Accept",
            vec!["application/json", "text/event-stream"],
        ))
        .respond_with(ResponseTemplate::new(202))
        .expect(1)
        .mount(&mock_server)
        .await;

    let transport = HttpTransport::new(&format!("{}/mcp", mock_server.uri())).unwrap();
    transport.connect().await.unwrap();

    let notification = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
        "params": {}
    });
    let _ = transport.send_notification(notification).await;
    // wiremock expect(1) will fail if Accept header is missing
}

#[tokio::test]
async fn test_mcp_protocol_version_header_on_requests() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(header("MCP-Protocol-Version", "2025-11-25"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(json_rpc_ok()),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let transport = HttpTransport::new(&format!("{}/mcp", mock_server.uri())).unwrap();
    transport.connect().await.unwrap();

    let _ = transport.send_request(ping_request()).await;
}

// ---------------------------------------------------------------------------
// Auth-header override (v0.3.44): rotating the bearer on a live transport.
//
// Regression net for the v0.3.43 observation that an OAuth `client_credentials`
// rotation could leave `disconnect()`'s DELETE flying under the *old* bearer
// (which the AS or upstream authorizer may have already revoked, surfacing as
// HTTP 403 Forbidden in ~15 ms). The fix exposes
// `Transport::update_auth_header()` / `McpClient::set_bearer()` so callers can
// rotate the bearer *before* DELETE without rebuilding the transport (which
// would drop the connection pool).
//
// These are wire-layer tests: they assert what reqwest actually puts on the
// wire (transport-protocol-boundary bytes), not framework-internal state.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_send_delete_uses_overridden_bearer_after_rotation() {
    let mock_server = MockServer::start().await;

    // The DELETE under the NEW bearer is the one we expect.
    Mock::given(method("DELETE"))
        .and(header("Authorization", "Bearer NEW"))
        .and(header("Mcp-Session-Id", "sess-abc"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Build the transport with the OLD bearer baked into `default_headers` —
    // the exact construction-time-only state v0.3.43 had no API to update.
    let mut headers = std::collections::HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer OLD".to_string());
    let config = ConnectionConfig {
        headers: Some(headers),
        ..Default::default()
    };
    let transport =
        HttpTransport::with_config(&format!("{}/mcp", mock_server.uri()), &config).unwrap();
    transport.connect().await.unwrap();

    // Rotate the bearer before disconnect — this is the new API.
    transport
        .update_auth_header(Some("Bearer NEW".to_string()))
        .await;

    transport.send_delete("sess-abc").await.unwrap();

    // wiremock verifies expect(1) with the NEW-bearer matcher on drop.
}

#[tokio::test]
async fn test_send_request_uses_overridden_bearer_after_rotation() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(header("Authorization", "Bearer NEW"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(json_rpc_ok()),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let mut headers = std::collections::HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer OLD".to_string());
    let config = ConnectionConfig {
        headers: Some(headers),
        ..Default::default()
    };
    let transport =
        HttpTransport::with_config(&format!("{}/mcp", mock_server.uri()), &config).unwrap();
    transport.connect().await.unwrap();

    transport
        .update_auth_header(Some("Bearer NEW".to_string()))
        .await;

    let _ = transport.send_request(ping_request()).await;
}

#[tokio::test]
async fn test_clearing_override_falls_back_to_default_headers() {
    let mock_server = MockServer::start().await;

    // After clearing, default_headers should reassert (Bearer OLD).
    Mock::given(method("POST"))
        .and(header("Authorization", "Bearer OLD"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(json_rpc_ok()),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let mut headers = std::collections::HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer OLD".to_string());
    let config = ConnectionConfig {
        headers: Some(headers),
        ..Default::default()
    };
    let transport =
        HttpTransport::with_config(&format!("{}/mcp", mock_server.uri()), &config).unwrap();
    transport.connect().await.unwrap();

    // Set, then clear — the next POST must carry the original default header.
    transport
        .update_auth_header(Some("Bearer NEW".to_string()))
        .await;
    transport.update_auth_header(None).await;

    let _ = transport.send_request(ping_request()).await;
}

// ============================================================================
// JSON-RPC envelope wire-format tests (turul-rpc typed constructors)
//
// These tests guard the refactor that replaced 20+ hand-rolled
// `json!({"jsonrpc": "2.0", ...})` envelopes with `JsonRpcRequest::new` /
// `JsonRpcNotification::new` from `turul-rpc`. Wiremock captures the actual
// bytes that hit the wire and the tests assert envelope shape, proving the
// typed constructors produce wire-format-compliant JSON-RPC 2.0 frames. The
// scope is the bytes consumed by the next protocol layer (the MCP server),
// not just the in-process `Value` the helper returns.
// ============================================================================

use turul_mcp_client::McpClient;
use turul_mcp_client::config::ClientConfig;
use turul_rpc::{JsonRpcNotification, JsonRpcRequest, RequestId, RequestParams};
use wiremock::matchers::body_partial_json;

#[tokio::test]
async fn test_typed_request_serializes_to_compliant_jsonrpc_envelope_on_wire() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(json_rpc_ok()),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let transport = HttpTransport::new(&format!("{}/mcp", mock_server.uri())).unwrap();
    transport.connect().await.unwrap();

    // Build a request via the same typed constructor McpClient::build_request uses.
    let mut params = std::collections::HashMap::new();
    params.insert("cursor".to_string(), serde_json::json!("page-2"));
    let request = JsonRpcRequest::new(
        RequestId::String("req_0".to_string()),
        "tools/list".to_string(),
        Some(RequestParams::Object(params)),
    );
    let envelope = serde_json::to_value(&request).unwrap();

    transport
        .send_request(envelope)
        .await
        .expect("send_request must reach the wiremock server");

    // Capture the actual bytes that hit the wire.
    let received = mock_server.received_requests().await.unwrap();
    assert_eq!(received.len(), 1);
    let body: serde_json::Value =
        serde_json::from_slice(&received[0].body).expect("wire body must be valid JSON");

    assert_eq!(
        body["jsonrpc"],
        serde_json::json!("2.0"),
        "typed constructor must emit `jsonrpc: \"2.0\"` envelope field"
    );
    assert_eq!(body["method"], serde_json::json!("tools/list"));
    assert_eq!(body["id"], serde_json::json!("req_0"));
    assert_eq!(body["params"]["cursor"], serde_json::json!("page-2"));
}

#[tokio::test]
async fn test_typed_request_with_empty_object_params_preserves_field_on_wire() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(json_rpc_ok()),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let transport = HttpTransport::new(&format!("{}/mcp", mock_server.uri())).unwrap();
    transport.connect().await.unwrap();

    // Empty Object params — must round-trip to `"params":{}` on the wire,
    // matching the prior hand-rolled `json!({"params": {}})` form. Some MCP
    // servers may treat an absent `params` field differently from an empty
    // object, so this contract is wire-byte-significant.
    let request = JsonRpcRequest::new(
        RequestId::String("req_0".to_string()),
        "ping".to_string(),
        Some(RequestParams::Object(std::collections::HashMap::new())),
    );
    let envelope = serde_json::to_value(&request).unwrap();

    transport
        .send_request(envelope)
        .await
        .expect("send_request must reach the wiremock server");

    let received = mock_server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&received[0].body).unwrap();

    assert_eq!(body["jsonrpc"], serde_json::json!("2.0"));
    assert_eq!(body["method"], serde_json::json!("ping"));
    assert!(
        body.get("params").is_some(),
        "Some(RequestParams::Object(empty)) MUST serialize as `\"params\":{{}}` on the wire, not omitted"
    );
    assert_eq!(body["params"], serde_json::json!({}));
}

/// End-to-end production-path coverage: drive the actual `McpClient` API
/// (`connect()` → `ping()`) against a wiremock server and assert that the
/// `ping` POST body on the wire matches the JSON-RPC 2.0 envelope shape
/// `build_request` is supposed to produce. Unlike the two tests above (which
/// construct `JsonRpcRequest` directly), this test walks the full path
/// `McpClient::ping → build_request → send_request_internal → HttpTransport
/// ::send_request → reqwest → wire`, so a regression that bypassed the
/// typed constructor in any single MCP method would be caught here.
#[tokio::test]
async fn test_mcp_client_ping_sends_typed_jsonrpc_envelope_through_full_stack() {
    let mock_server = MockServer::start().await;

    // SSE listener fires a GET on connect — return 404 so the listener exits
    // cleanly (per v0.3.38 4xx-terminal contract) without disrupting the test.
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    // server/discover probe: a 2025-11-25 server answers -32601, driving the
    // bilingual client to fall back to the initialize handshake.
    Mock::given(method("POST"))
        .and(body_partial_json(serde_json::json!({"method": "server/discover"})))
        .respond_with(ResponseTemplate::new(200).insert_header("Content-Type", "application/json").set_body_json(
            serde_json::json!({"jsonrpc": "2.0", "id": "req_0", "error": {"code": -32601, "message": "Method not found"}}),
        ))
        .mount(&mock_server)
        .await;

    // initialize: return a session ID and a minimal valid InitializeResult.
    Mock::given(method("POST"))
        .and(body_partial_json(
            serde_json::json!({"method": "initialize"}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .insert_header("Mcp-Session-Id", "sess-wire-test")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": "req_0",
                    "result": {
                        "protocolVersion": "2025-11-25",
                        // listChanged advertised as false: this static mock does NOT
                        // emit `notifications/tools/list_changed`, and a server MUST NOT
                        // claim a capability it does not actually deliver.
                        "capabilities": { "tools": { "listChanged": false } },
                        "serverInfo": { "name": "wire-mock", "version": "1.0.0" }
                    }
                })),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    // notifications/initialized: 202 Accepted per MCP spec.
    Mock::given(method("POST"))
        .and(body_partial_json(
            serde_json::json!({"method": "notifications/initialized"}),
        ))
        .respond_with(ResponseTemplate::new(202).insert_header("Content-Type", "application/json"))
        .expect(1)
        .mount(&mock_server)
        .await;

    // ping: the call we actually want to inspect on the wire.
    Mock::given(method("POST"))
        .and(body_partial_json(serde_json::json!({"method": "ping"})))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0", "id": "req_1", "result": {}
                })),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    // disconnect fires DELETE on Drop / explicit disconnect.
    Mock::given(method("DELETE"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let url = format!("{}/mcp", mock_server.uri());
    let transport = Box::new(HttpTransport::new(&url).unwrap());
    let client = McpClient::new(transport, ClientConfig::default());

    client
        .connect()
        .await
        .expect("connect() must succeed against the wiremock server");
    client.ping().await.expect("ping() must succeed");

    // Find the `ping` POST among captured requests and assert the envelope.
    let received = mock_server.received_requests().await.unwrap();
    let ping_req = received
        .iter()
        .find(|r| {
            let body: serde_json::Value =
                serde_json::from_slice(&r.body).unwrap_or(serde_json::Value::Null);
            body.get("method") == Some(&serde_json::json!("ping"))
        })
        .expect("expected to capture a `ping` POST on the wire");

    let body: serde_json::Value =
        serde_json::from_slice(&ping_req.body).expect("ping body must be valid JSON");

    assert_eq!(
        body["jsonrpc"],
        serde_json::json!("2.0"),
        "McpClient::ping must emit the `jsonrpc: \"2.0\"` envelope field"
    );
    assert_eq!(body["method"], serde_json::json!("ping"));
    assert_eq!(
        body["params"],
        serde_json::json!({}),
        "ping() passes json!({{}}) which must round-trip to `\"params\":{{}}` on the wire"
    );
    assert!(
        body["id"].is_string(),
        "ping() must carry an `id` field (it's a request, not a notification); got: {:?}",
        body.get("id")
    );

    // Also confirm the `notifications/initialized` POST did NOT carry an id —
    // proves the swept `build_notification` helper takes the production path.
    let init_notif = received
        .iter()
        .find(|r| {
            let body: serde_json::Value =
                serde_json::from_slice(&r.body).unwrap_or(serde_json::Value::Null);
            body.get("method") == Some(&serde_json::json!("notifications/initialized"))
        })
        .expect("expected to capture a `notifications/initialized` POST");
    let init_body: serde_json::Value = serde_json::from_slice(&init_notif.body).unwrap();
    assert_eq!(init_body["jsonrpc"], serde_json::json!("2.0"));
    assert!(
        init_body.get("id").is_none(),
        "notifications/initialized MUST NOT carry an `id` field per JSON-RPC 2.0 §4.1; got: {:?}",
        init_body.get("id")
    );
}

/// Regression: MCP tool arguments routinely carry array values
/// (e.g., `{"values": [1, 2, 3]}`). This test proves array-valued arguments
/// survive end-to-end through `McpClient::call_tool` → `build_request` →
/// typed `JsonRpcRequest` → wire bytes. It mocks `tools/call`, captures the
/// actual POST body, and asserts the array sits intact at
/// `params.arguments.values` — confirming that routing through
/// `RequestParams::Object(HashMap<String, Value>)` does NOT flatten, coerce,
/// or otherwise mangle nested array values. Distinct from JSON-RPC envelope
/// positional params (which MCP never uses at the `params` level).
#[tokio::test]
async fn test_mcp_client_call_tool_preserves_array_argument_values_on_wire() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    // server/discover probe: a 2025-11-25 server answers -32601, driving the
    // bilingual client to fall back to the initialize handshake.
    Mock::given(method("POST"))
        .and(body_partial_json(serde_json::json!({"method": "server/discover"})))
        .respond_with(ResponseTemplate::new(200).insert_header("Content-Type", "application/json").set_body_json(
            serde_json::json!({"jsonrpc": "2.0", "id": "req_0", "error": {"code": -32601, "message": "Method not found"}}),
        ))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(body_partial_json(
            serde_json::json!({"method": "initialize"}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .insert_header("Mcp-Session-Id", "sess-array-test")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": "req_0",
                    "result": {
                        "protocolVersion": "2025-11-25",
                        // listChanged advertised as false: this static mock does NOT
                        // emit `notifications/tools/list_changed`, and a server MUST NOT
                        // claim a capability it does not actually deliver.
                        "capabilities": { "tools": { "listChanged": false } },
                        "serverInfo": { "name": "wire-mock", "version": "1.0.0" }
                    }
                })),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(body_partial_json(
            serde_json::json!({"method": "notifications/initialized"}),
        ))
        .respond_with(ResponseTemplate::new(202))
        .expect(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(body_partial_json(
            serde_json::json!({"method": "tools/call"}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": "req_1",
                    "result": {
                        "content": [{"type": "text", "text": "ok"}],
                        "isError": false
                    }
                })),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("DELETE"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let url = format!("{}/mcp", mock_server.uri());
    let transport = Box::new(HttpTransport::new(&url).unwrap());
    let client = McpClient::new(transport, ClientConfig::default());
    client.connect().await.expect("connect must succeed");

    // The regression case: array-valued tool argument inside the
    // `arguments` Object (the common MCP shape, not JSON-RPC positional params).
    client
        .call_tool(
            "compute_stats",
            serde_json::json!({
                "values": [1, 2, 3, 4, 5],
                "tags": ["alpha", "beta"],
                "matrix": [[1, 2], [3, 4]],
            }),
        )
        .await
        .expect("call_tool must succeed against the wiremock server");

    let received = mock_server.received_requests().await.unwrap();
    let call_req = received
        .iter()
        .find(|r| {
            let body: serde_json::Value =
                serde_json::from_slice(&r.body).unwrap_or(serde_json::Value::Null);
            body.get("method") == Some(&serde_json::json!("tools/call"))
        })
        .expect("expected to capture a `tools/call` POST on the wire");

    let body: serde_json::Value =
        serde_json::from_slice(&call_req.body).expect("body must be valid JSON");

    // Envelope.
    assert_eq!(body["jsonrpc"], serde_json::json!("2.0"));
    assert_eq!(body["method"], serde_json::json!("tools/call"));

    // Params shape — `name` and `arguments` at the MCP layer, NOT positional.
    assert_eq!(body["params"]["name"], serde_json::json!("compute_stats"));
    assert!(
        body["params"]["arguments"].is_object(),
        "params.arguments must be an Object (MCP uses named args, not positional); got: {:?}",
        body["params"]["arguments"]
    );

    // The actual regression assertion: array values inside arguments must be
    // preserved verbatim on the wire — same length, same element types, no
    // stringification, no flattening of nested arrays.
    assert_eq!(
        body["params"]["arguments"]["values"],
        serde_json::json!([1, 2, 3, 4, 5]),
        "numeric array argument must round-trip through RequestParams::Object intact"
    );
    assert_eq!(
        body["params"]["arguments"]["tags"],
        serde_json::json!(["alpha", "beta"]),
        "string array argument must round-trip intact"
    );
    assert_eq!(
        body["params"]["arguments"]["matrix"],
        serde_json::json!([[1, 2], [3, 4]]),
        "nested-array argument must round-trip intact (no flattening)"
    );
}

#[tokio::test]
async fn test_typed_notification_omits_id_field_on_wire() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(202).insert_header("Content-Type", "application/json"))
        .expect(1)
        .mount(&mock_server)
        .await;

    let transport = HttpTransport::new(&format!("{}/mcp", mock_server.uri())).unwrap();
    transport.connect().await.unwrap();

    let notification = JsonRpcNotification::new(
        "notifications/initialized".to_string(),
        Some(RequestParams::Object(std::collections::HashMap::new())),
    );
    let envelope = serde_json::to_value(&notification).unwrap();

    transport
        .send_notification(envelope)
        .await
        .expect("send_notification must reach the wiremock server");

    let received = mock_server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&received[0].body).unwrap();

    assert_eq!(body["jsonrpc"], serde_json::json!("2.0"));
    assert_eq!(
        body["method"],
        serde_json::json!("notifications/initialized")
    );
    assert!(
        body.get("id").is_none(),
        "JSON-RPC 2.0 §4.1 — notifications MUST NOT contain an `id` field on the wire"
    );
}
