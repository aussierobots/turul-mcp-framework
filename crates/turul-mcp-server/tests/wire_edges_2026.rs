//! Wire-level edge acceptance on the 2026 path: request-id and `_meta`
//! validation branches, version hints on `initialize`, list determinism +
//! pagination + cacheable fields, error-code mapping for unknown
//! tools/prompts/resources, blob encoding validation, and prompt-descriptor
//! fidelity.
//!
//! Built only under the 2026 feature; compiles to nothing under 2025-11-25.
#![cfg(feature = "protocol-2026-07-28")]

use std::collections::HashMap;

use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;

#[derive(McpTool, Clone, Default)]
#[tool(name = "alpha_tool", description = "First tool", output = String)]
struct AlphaTool {}
impl AlphaTool {
    async fn execute(&self, _s: Option<SessionContext>) -> McpResult<String> {
        Ok("a".into())
    }
}

#[derive(McpTool, Clone, Default)]
#[tool(name = "beta_tool", description = "Second tool", output = String)]
struct BetaTool {}
impl BetaTool {
    async fn execute(&self, _s: Option<SessionContext>) -> McpResult<String> {
        Ok("b".into())
    }
}

#[derive(McpTool, Clone, Default)]
#[tool(name = "gamma_tool", description = "Third tool", output = String)]
struct GammaTool {}
impl GammaTool {
    async fn execute(&self, _s: Option<SessionContext>) -> McpResult<String> {
        Ok("c".into())
    }
}

/// Prompt carrying title + icons + _meta — prompts/list must not drop them.
struct TitledPrompt;

impl turul_mcp_server::prelude::HasPromptMetadata for TitledPrompt {
    fn name(&self) -> &str {
        "titled"
    }
    fn title(&self) -> Option<&str> {
        Some("A Titled Prompt")
    }
}
impl turul_mcp_server::prelude::HasPromptDescription for TitledPrompt {}
impl turul_mcp_server::prelude::HasPromptArguments for TitledPrompt {}
impl turul_mcp_server::prelude::HasPromptAnnotations for TitledPrompt {}
impl turul_mcp_server::prelude::HasPromptMeta for TitledPrompt {
    fn prompt_meta(&self) -> Option<&HashMap<String, serde_json::Value>> {
        use std::sync::OnceLock;
        static META: OnceLock<HashMap<String, serde_json::Value>> = OnceLock::new();
        Some(META.get_or_init(|| {
            let mut m = HashMap::new();
            m.insert("example.com/tag".to_string(), serde_json::json!("v1"));
            m
        }))
    }
}
impl turul_mcp_server::prelude::HasIcons for TitledPrompt {
    fn icons(&self) -> Option<&Vec<turul_mcp_protocol::Icon>> {
        use std::sync::OnceLock;
        static ICONS: OnceLock<Vec<turul_mcp_protocol::Icon>> = OnceLock::new();
        Some(ICONS.get_or_init(|| vec![turul_mcp_protocol::Icon::new("https://example.com/p.png")]))
    }
}

#[async_trait::async_trait]
impl turul_mcp_server::McpPrompt for TitledPrompt {
    async fn render(
        &self,
        _args: Option<HashMap<String, serde_json::Value>>,
    ) -> McpResult<Vec<turul_mcp_protocol::prompts::PromptMessage>> {
        Ok(vec![turul_mcp_protocol::prompts::PromptMessage::user_text(
            "hello",
        )])
    }
}

/// Resource whose blob is NOT valid base64 — the read path must reject it.
struct BadBlobResource;

impl turul_mcp_server::prelude::HasResourceMetadata for BadBlobResource {
    fn name(&self) -> &str {
        "badblob"
    }
}
impl turul_mcp_server::prelude::HasResourceDescription for BadBlobResource {}
impl turul_mcp_server::prelude::HasResourceUri for BadBlobResource {
    fn uri(&self) -> &str {
        "file:///bad.bin"
    }
}
impl turul_mcp_server::prelude::HasResourceMimeType for BadBlobResource {}
impl turul_mcp_server::prelude::HasResourceSize for BadBlobResource {}
impl turul_mcp_server::prelude::HasResourceAnnotations for BadBlobResource {}
impl turul_mcp_server::prelude::HasResourceMeta for BadBlobResource {}
impl turul_mcp_server::prelude::HasIcons for BadBlobResource {}

#[async_trait::async_trait]
impl turul_mcp_server::McpResource for BadBlobResource {
    async fn read(
        &self,
        _params: Option<serde_json::Value>,
        _session: Option<&SessionContext>,
    ) -> McpResult<Vec<turul_mcp_protocol::resources::ResourceContent>> {
        Ok(vec![turul_mcp_protocol::resources::ResourceContent::blob(
            "file:///bad.bin",
            "this is !!! not base64 @@@",
            "application/octet-stream".to_string(),
        )])
    }
}

async fn start_server() -> String {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();

    let server = McpServer::builder()
        .name("wire-edges-2026")
        .version("0.4.0")
        .tool(AlphaTool::default())
        .tool(BetaTool::default())
        .tool(GammaTool::default())
        .prompt(TitledPrompt)
        .resource(BadBlobResource)
        .test_mode() // resource security allowlist off — the blob test targets encoding, not ACLs
        .bind_address(format!("127.0.0.1:{port}").parse().unwrap())
        .build()
        .expect("build 2026 server");

    tokio::spawn(async move {
        server.run().await.ok();
    });

    let url = format!("http://127.0.0.1:{port}/mcp");
    let client = reqwest::Client::new();
    for _ in 0..50 {
        if client.get(&url).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    url
}

fn meta() -> serde_json::Value {
    serde_json::json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "test-client", "version": "1.0.0" },
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

async fn post_raw(
    url: &str,
    rpc_method: &str,
    name_header: Option<&str>,
    body: serde_json::Value,
) -> (reqwest::StatusCode, serde_json::Value) {
    let client = reqwest::Client::new();
    let mut req = client
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", rpc_method);
    if let Some(n) = name_header {
        req = req.header("Mcp-Name", n);
    }
    let resp = req.json(&body).send().await.expect("POST");
    let status = resp.status();
    let json: serde_json::Value = resp.json().await.unwrap_or_default();
    (status, json)
}

async fn post_method(
    url: &str,
    rpc_method: &str,
    name_header: Option<&str>,
    params: serde_json::Value,
) -> (reqwest::StatusCode, serde_json::Value) {
    post_raw(
        url,
        rpc_method,
        name_header,
        serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": rpc_method, "params": params
        }),
    )
    .await
}

/// "Unlike base JSON-RPC, the ID MUST NOT be null."
#[tokio::test]
async fn null_request_id_is_rejected() {
    let url = start_server().await;
    let (status, body) = post_raw(
        &url,
        "tools/list",
        None,
        serde_json::json!({
            "jsonrpc": "2.0", "id": null, "method": "tools/list",
            "params": { "_meta": meta() }
        }),
    )
    .await;
    assert_eq!(status, 400, "null id must be rejected: {body}");
    assert_eq!(body["error"]["code"], -32600, "{body}");
}

/// `_meta` present but clientInfo / protocolVersion absent → -32602.
#[tokio::test]
async fn incomplete_meta_branches_are_rejected() {
    let url = start_server().await;
    for (label, bad_meta) in [
        (
            "missing clientInfo",
            serde_json::json!({
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientCapabilities": {}
            }),
        ),
        (
            "missing protocolVersion",
            serde_json::json!({
                "io.modelcontextprotocol/clientInfo": { "name": "t", "version": "1" },
                "io.modelcontextprotocol/clientCapabilities": {}
            }),
        ),
    ] {
        let (status, body) = post_method(
            &url,
            "tools/list",
            None,
            serde_json::json!({ "_meta": bad_meta }),
        )
        .await;
        assert_eq!(status, 400, "{label}: {body}");
        assert_eq!(body["error"]["code"], -32602, "{label}: {body}");
    }
}

/// Versioning §Backward Compatibility: errors to `initialize` SHOULD name
/// the supported protocol versions — a legacy client's only diagnostic.
#[tokio::test]
async fn initialize_error_names_supported_versions() {
    let url = start_server().await;
    let (status, body) = post_method(
        &url,
        "initialize",
        None,
        serde_json::json!({
            "protocolVersion": "2025-11-25", "capabilities": {},
            "clientInfo": { "name": "legacy", "version": "1.0" },
            "_meta": meta()
        }),
    )
    .await;
    assert_eq!(status, 404);
    assert_eq!(body["error"]["code"], -32601, "{body}");
    assert_eq!(
        body["error"]["data"]["supported"],
        serde_json::json!(["2026-07-28"]),
        "the initialize rejection must name the supported versions: {body}"
    );
}

/// Unknown tool → -32602 on the real wire (not just the protocol-crate unit).
#[tokio::test]
async fn unknown_tool_is_invalid_params_on_the_wire() {
    let url = start_server().await;
    let (_, body) = post_method(
        &url,
        "tools/call",
        Some("no_such_tool"),
        serde_json::json!({ "name": "no_such_tool", "arguments": {}, "_meta": meta() }),
    )
    .await;
    assert_eq!(body["error"]["code"], -32602, "{body}");
}

/// tools/list: deterministic order across calls, cursor pagination walk,
/// invalid cursor → -32602, and the CacheableResult fields on the wire.
#[tokio::test]
async fn tools_list_is_deterministic_paginated_and_cacheable() {
    let url = start_server().await;

    let (_, first) = post_method(
        &url,
        "tools/list",
        None,
        serde_json::json!({"_meta": meta()}),
    )
    .await;
    let (_, second) = post_method(
        &url,
        "tools/list",
        None,
        serde_json::json!({"_meta": meta()}),
    )
    .await;
    assert_eq!(
        first["result"]["tools"], second["result"]["tools"],
        "tools/list must be deterministic across calls"
    );
    let names: Vec<&str> = first["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "stable name ordering");
    assert!(first["result"]["ttlMs"].is_number(), "{first}");
    assert!(first["result"]["cacheScope"].is_string(), "{first}");

    // Cursor walk with limit=1 visits every tool exactly once, in order.
    let mut walked = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let mut params = serde_json::json!({ "limit": 1, "_meta": meta() });
        if let Some(c) = &cursor {
            params["cursor"] = serde_json::json!(c);
        }
        let (_, page) = post_method(&url, "tools/list", None, params).await;
        for t in page["result"]["tools"].as_array().unwrap() {
            walked.push(t["name"].as_str().unwrap().to_string());
        }
        match page["result"]["nextCursor"].as_str() {
            Some(next) => cursor = Some(next.to_string()),
            None => break,
        }
    }
    assert_eq!(walked, sorted, "pagination walk covers every tool once");

    // Invalid cursor → -32602 ("Invalid cursors SHOULD result in an error").
    let (_, body) = post_method(
        &url,
        "tools/list",
        None,
        serde_json::json!({ "cursor": "garbage-cursor", "_meta": meta() }),
    )
    .await;
    assert_eq!(body["error"]["code"], -32602, "{body}");
}

/// "Result responses MUST include the same ID as the request they correspond
/// to" (basic §Requests). Prior evidence only traced the *error*-response
/// echo path (`JsonRpcError::new(Some(req.id.clone()), ...)`); this drives a
/// genuinely successful `tools/list` and asserts the success-response id
/// matches a distinctive, non-default request id on the real wire.
#[tokio::test]
async fn success_response_echoes_the_request_id() {
    let url = start_server().await;
    let (status, body) = post_raw(
        &url,
        "tools/list",
        None,
        serde_json::json!({
            "jsonrpc": "2.0", "id": 987654, "method": "tools/list",
            "params": { "_meta": meta() }
        }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert!(body.get("error").is_none(), "{body}");
    assert_eq!(
        body["id"], 987654,
        "success response id must echo the request id: {body}"
    );
}

/// "JSON-RPC messages MUST be UTF-8 encoded" (basic/transports). The server
/// must reject a POST body containing invalid UTF-8 bytes with HTTP 400
/// rather than attempting to parse it as JSON-RPC.
#[tokio::test]
async fn non_utf8_post_body_is_rejected_with_400() {
    let url = start_server().await;
    let client = reqwest::Client::new();

    // 0xFF is not a valid UTF-8 continuation/lead byte in this position.
    let invalid_utf8: &[u8] = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{\"x\":\"\xff\xfe\"}}";

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/list")
        .body(invalid_utf8.to_vec())
        .send()
        .await
        .expect("POST with invalid UTF-8 body");

    assert_eq!(
        resp.status(),
        400,
        "a non-UTF-8 POST body must be rejected with 400"
    );
    let body: serde_json::Value = resp.json().await.expect("json error body");
    let message = body["error"]["message"].as_str().unwrap_or_default();
    assert!(
        message.to_ascii_lowercase().contains("utf-8"),
        "error message should name the UTF-8 violation: {body}"
    );
}

async fn list_tools_on(client: &reqwest::Client, url: &str) -> serde_json::Value {
    let resp = client
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/list")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/list",
            "params": { "_meta": meta() }
        }))
        .send()
        .await
        .expect("tools/list POST");
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.expect("json body");
    body["result"]["tools"].clone()
}

/// "Tool set MUST NOT vary per-connection" (server/tools) — two entirely
/// independent `reqwest::Client` instances (separate connection pools, so
/// genuinely separate TCP connections, not just separate logical calls) must
/// see the identical tool set from the same running server.
#[tokio::test]
async fn tools_list_is_invariant_across_independent_connections() {
    let url = start_server().await;

    let client_a = reqwest::Client::new();
    let client_b = reqwest::Client::new();

    let from_a = list_tools_on(&client_a, &url).await;
    let from_b = list_tools_on(&client_b, &url).await;

    assert_eq!(
        from_a, from_b,
        "tools/list must not vary across independent client connections"
    );
}

/// "Tool set MUST NOT vary ... as a side effect of other requests" (server/tools).
/// Two `tools/list` snapshots on one client bracket an intervening, unrelated
/// `tools/call` — proving that request has no side effect on the advertised
/// tool set. (This does not by itself prove per-connection invariance, since
/// `reqwest::Client` pools rather than guarantees one fixed TCP connection —
/// that half of the requirement is covered separately, above.)
#[tokio::test]
async fn tools_list_is_unchanged_by_an_intervening_unrelated_request() {
    let url = start_server().await;
    let client = reqwest::Client::new();

    let before = list_tools_on(&client, &url).await;

    let call_resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "alpha_tool")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": { "_meta": meta(), "name": "alpha_tool", "arguments": {} }
        }))
        .send()
        .await
        .expect("tools/call POST");
    assert_eq!(call_resp.status(), 200);

    let after = list_tools_on(&client, &url).await;

    assert_eq!(
        before, after,
        "tools/list must not vary as a side effect of an intervening tools/call"
    );
}

/// resources/read for a nonexistent URI → -32602 on the real wire (MUST).
#[tokio::test]
async fn nonexistent_resource_is_invalid_params_on_the_wire() {
    let url = start_server().await;
    let (_, body) = post_method(
        &url,
        "resources/read",
        Some("file:///nope.txt"),
        serde_json::json!({ "uri": "file:///nope.txt", "_meta": meta() }),
    )
    .await;
    assert_eq!(body["error"]["code"], -32602, "{body}");
}

/// "Binary data MUST be properly encoded" — a provider returning a non-base64
/// blob is a server-side error, not a silent invalid payload.
#[tokio::test]
async fn invalid_base64_blob_is_rejected() {
    let url = start_server().await;
    let (_, body) = post_method(
        &url,
        "resources/read",
        Some("file:///bad.bin"),
        serde_json::json!({ "uri": "file:///bad.bin", "_meta": meta() }),
    )
    .await;
    assert!(
        body.get("error").is_some(),
        "invalid base64 must not ship: {body}"
    );
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("base64"),
        "{body}"
    );
}

/// prompts/list carries title, icons, and _meta (Prompt extends
/// BaseMetadata, Icons); prompts/get for an unknown prompt → -32602; a
/// mismatched Mcp-Name on prompts/get → -32020 + 400.
#[tokio::test]
async fn prompt_descriptors_and_error_codes() {
    let url = start_server().await;

    let (_, body) = post_method(
        &url,
        "prompts/list",
        None,
        serde_json::json!({"_meta": meta()}),
    )
    .await;
    let prompt = &body["result"]["prompts"][0];
    assert_eq!(prompt["name"], "titled", "{body}");
    assert_eq!(
        prompt["title"], "A Titled Prompt",
        "title must survive: {body}"
    );
    assert_eq!(
        prompt["icons"][0]["src"], "https://example.com/p.png",
        "icons must survive: {body}"
    );
    assert_eq!(
        prompt["_meta"]["example.com/tag"], "v1",
        "_meta must survive: {body}"
    );

    let (_, body) = post_method(
        &url,
        "prompts/get",
        Some("no_such_prompt"),
        serde_json::json!({ "name": "no_such_prompt", "_meta": meta() }),
    )
    .await;
    assert_eq!(body["error"]["code"], -32602, "unknown prompt: {body}");

    // Mcp-Name MUST equal params.name (SEP-2243) — mismatch is -32020/400.
    let (status, body) = post_method(
        &url,
        "prompts/get",
        Some("titled"),
        serde_json::json!({ "name": "different", "_meta": meta() }),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert_eq!(body["error"]["code"], -32020, "{body}");
}

/// An unconfigured server answers completion/complete with 404 + -32601
/// ("Servers SHOULD return -32601 when completion is unsupported").
#[tokio::test]
async fn completion_unsupported_is_method_not_found() {
    let url = start_server().await; // no completion providers registered
    let (status, body) = post_method(
        &url,
        "completion/complete",
        None,
        serde_json::json!({
            "ref": { "type": "ref/prompt", "name": "titled" },
            "argument": { "name": "a", "value": "" },
            "_meta": meta()
        }),
    )
    .await;
    assert_eq!(status, 404, "{body}");
    assert_eq!(body["error"]["code"], -32601, "{body}");
}

/// "Servers that emit log message notifications MUST declare the logging
/// capability" — any tool may call notify_log, so the capability is
/// advertised; request-stream delivery is gated per-request by logLevel.
#[tokio::test]
async fn discover_declares_the_logging_capability() {
    let url = start_server().await;
    let (_, body) = post_method(
        &url,
        "server/discover",
        None,
        serde_json::json!({"_meta": meta()}),
    )
    .await;
    assert!(
        body["result"]["capabilities"]["logging"].is_object(),
        "{body}"
    );
}
