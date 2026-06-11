//! Wire-level acceptance for the DRAFT-2026-v1 stateless server core.
//!
//! Proves, against a real HTTP server built for the 2026 spec, that:
//!   1. `server/discover` answers without any session and returns a wire-shaped
//!      `DiscoverResult` (`resultType: "complete"`, `supportedVersions`,
//!      `capabilities`, `serverInfo`).
//!   2. `tools/call` dispatches with NO `Mcp-Session-Id` and NO prior
//!      `initialize`/`initialized` handshake — the stateless core never answers
//!      a sessionless request with HTTP 400.
//!
//! Built only under the 2026 feature; compiles to nothing under 2025-11-25.
#![cfg(feature = "protocol-2026-07-28")]

use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;

#[derive(McpTool, Clone, Default)]
#[tool(name = "echo", description = "Echo back the provided message", output = String)]
struct EchoTool {
    #[param(description = "Message to echo back")]
    message: String,
}

impl EchoTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok(format!("Echo: {}", self.message))
    }
}

/// Start a 2026 server on an ephemeral port and return its `/mcp` URL once it
/// accepts connections.
async fn start_server() -> String {
    // Reserve a free port, then hand it to the server. The brief gap between
    // dropping this listener and the server binding is the standard test pattern.
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();

    let server = McpServer::builder()
        .name("discover-2026-test")
        .version("0.4.0")
        .tool(EchoTool::default())
        .with_resources()
        .with_prompts()
        .bind_address(format!("127.0.0.1:{port}").parse().unwrap())
        .build()
        .expect("build 2026 server");

    tokio::spawn(async move {
        server.run().await.ok();
    });

    let url = format!("http://127.0.0.1:{port}/mcp");
    // Wait until the accept loop is live (build() binds; run() starts accepting).
    let client = reqwest::Client::new();
    for _ in 0..50 {
        if client.get(&url).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    url
}

/// A spec-complete per-request `RequestMetaObject` — the 2026 core requires
/// `protocolVersion`, `clientInfo`, and `clientCapabilities` on every request.
fn meta() -> serde_json::Value {
    serde_json::json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "test-client", "version": "1.0.0" },
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

#[tokio::test]
async fn server_discover_answers_without_a_session() {
    let url = start_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "server/discover")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "server/discover",
            "params": { "_meta": meta() }
        }))
        .send()
        .await
        .expect("server/discover POST");

    assert_eq!(
        resp.status(),
        200,
        "server/discover must succeed without an Mcp-Session-Id"
    );
    // The server must advertise 2026-07-28 on the wire, not fall back to 2025-11-25.
    assert_eq!(
        resp.headers()
            .get("MCP-Protocol-Version")
            .and_then(|v| v.to_str().ok()),
        Some("2026-07-28"),
        "a 2026 server must echo MCP-Protocol-Version: 2026-07-28"
    );
    let body: serde_json::Value = resp.json().await.expect("json body");
    assert_eq!(body["result"]["resultType"], "complete");
    assert_eq!(
        body["result"]["supportedVersions"][0], "2026-07-28",
        "server must advertise the 2026 protocol version"
    );
    assert!(body["result"]["capabilities"].is_object());
    assert_eq!(body["result"]["serverInfo"]["name"], "discover-2026-test");
}

#[tokio::test]
async fn tools_call_dispatches_without_session_handshake() {
    let url = start_server().await;
    let client = reqwest::Client::new();

    // No initialize, no notifications/initialized, no Mcp-Session-Id header.
    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "echo")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": {
                "_meta": meta(),
                "name": "echo",
                "arguments": { "message": "hi" }
            }
        }))
        .send()
        .await
        .expect("tools/call POST");

    assert_eq!(
        resp.status(),
        200,
        "stateless tools/call must dispatch without a session (never HTTP 400)"
    );
    let body: serde_json::Value = resp.json().await.expect("json body");
    assert!(
        body.get("error").is_none(),
        "tools/call must not error on a sessionless 2026 request: {body}"
    );
    let text = body["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default();
    assert!(
        text.contains("Echo: hi"),
        "unexpected tool result shape: {body}"
    );
}

/// Sends a sessionless list request and returns the parsed JSON-RPC body.
async fn list_request(url: &str, rpc_method: &str) -> serde_json::Value {
    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", rpc_method)
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 9, "method": rpc_method,
            "params": { "_meta": meta() }
        }))
        .send()
        .await
        .unwrap_or_else(|_| panic!("{rpc_method} POST"));
    assert_eq!(
        resp.status(),
        200,
        "stateless {rpc_method} must dispatch without a session"
    );
    resp.json().await.expect("json body")
}

#[tokio::test]
async fn resources_list_dispatches_statelessly_with_cacheable_result() {
    let url = start_server().await;
    let body = list_request(&url, "resources/list").await;
    assert!(
        body.get("error").is_none(),
        "resources/list errored: {body}"
    );
    // 2026 list results extend CacheableResult.
    assert_eq!(body["result"]["resultType"], "complete");
    assert!(
        body["result"]["cacheScope"].is_string(),
        "missing cacheScope: {body}"
    );
    assert!(
        body["result"]["resources"].is_array(),
        "missing resources array: {body}"
    );
}

#[tokio::test]
async fn prompts_list_dispatches_statelessly_with_cacheable_result() {
    let url = start_server().await;
    let body = list_request(&url, "prompts/list").await;
    assert!(body.get("error").is_none(), "prompts/list errored: {body}");
    assert_eq!(body["result"]["resultType"], "complete");
    assert!(
        body["result"]["cacheScope"].is_string(),
        "missing cacheScope: {body}"
    );
    assert!(
        body["result"]["prompts"].is_array(),
        "missing prompts array: {body}"
    );
}

/// POST `body` (with valid standard headers), asserting the 2026 server
/// rejects it with HTTP 400 + JSON-RPC -32602.
async fn assert_rejected_invalid_params(url: &str, body: serde_json::Value) {
    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "echo")
        .json(&body)
        .send()
        .await
        .expect("POST");
    assert_eq!(resp.status(), 400, "must be rejected with HTTP 400: {body}");
    let out: serde_json::Value = resp.json().await.expect("json body");
    assert_eq!(
        out["error"]["code"], -32602,
        "must be JSON-RPC -32602: {out}"
    );
}

#[tokio::test]
async fn missing_meta_is_rejected_with_invalid_params() {
    let url = start_server().await;
    // 2026 requires params._meta on every request — a tools/call without it is invalid.
    assert_rejected_invalid_params(
        &url,
        serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": "echo", "arguments": { "message": "hi" } }
        }),
    )
    .await;
}

#[tokio::test]
async fn incomplete_meta_missing_client_capabilities_is_rejected() {
    let url = start_server().await;
    assert_rejected_invalid_params(
        &url,
        serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                    "io.modelcontextprotocol/clientInfo": { "name": "t", "version": "1.0.0" }
                    // missing io.modelcontextprotocol/clientCapabilities
                },
                "name": "echo", "arguments": { "message": "hi" }
            }
        }),
    )
    .await;
}

#[tokio::test]
async fn unsupported_protocol_version_header_is_rejected_with_32004() {
    let url = start_server().await;
    // A 2026-only build does not implement 2025-11-25: requesting it must get
    // 400 + UnsupportedProtocolVersionError listing the supported set.
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2025-11-25")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "echo")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "_meta": meta(), "name": "echo", "arguments": { "message": "hi" } }
        }))
        .send()
        .await
        .expect("POST");
    assert_eq!(resp.status(), 400);
    let out: serde_json::Value = resp.json().await.expect("json body");
    assert_eq!(out["error"]["code"], -32004, "must be -32004: {out}");
    assert_eq!(out["error"]["data"]["supported"][0], "2026-07-28");
    assert_eq!(out["error"]["data"]["requested"], "2025-11-25");
}

#[tokio::test]
async fn header_body_protocol_version_mismatch_is_rejected_with_32001() {
    let url = start_server().await;
    // Header carries the supported 2026-07-28, but _meta claims 2025-11-25:
    // a header-validation failure → 400 + -32001 HeaderMismatch.
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "echo")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2025-11-25",
                    "io.modelcontextprotocol/clientInfo": { "name": "t", "version": "1.0.0" },
                    "io.modelcontextprotocol/clientCapabilities": {}
                },
                "name": "echo", "arguments": { "message": "hi" }
            }
        }))
        .send()
        .await
        .expect("POST");
    assert_eq!(resp.status(), 400);
    let out: serde_json::Value = resp.json().await.expect("json body");
    assert_eq!(out["error"]["code"], -32001, "must be -32001: {out}");
}

#[tokio::test]
async fn tools_list_advertises_output_schema() {
    let url = start_server().await;
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/list")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 11, "method": "tools/list",
            "params": { "_meta": meta() }
        }))
        .send()
        .await
        .expect("tools/list POST");
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.expect("json body");
    let tools = body["result"]["tools"].as_array().expect("tools array");
    let echo = tools
        .iter()
        .find(|t| t["name"] == "echo")
        .expect("echo tool listed");
    // The derive macro declares `output = String`, so the tool HAS an output
    // schema — tools/list MUST advertise it (a tool with outputSchema must
    // return conforming structuredContent, and clients can only know that
    // contract if the list result carries the schema).
    assert!(
        echo["outputSchema"].is_object(),
        "tools/list must advertise outputSchema for tools that declare one: {echo}"
    );
}

#[tokio::test]
async fn completion_complete_dispatches_statelessly() {
    // completion/complete is part of the 2026 core; with completion enabled
    // the request must dispatch sessionless and return the CompleteResult
    // wire shape ({ completion: { values, ... } }).
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let server = McpServer::builder()
        .name("completion-2026-test")
        .version("0.4.0")
        .tool(EchoTool::default())
        .with_completion()
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

    // Capability must be advertised…
    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "server/discover")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 30, "method": "server/discover",
            "params": { "_meta": meta() }
        }))
        .send()
        .await
        .expect("discover POST");
    let body: serde_json::Value = resp.json().await.expect("json");
    assert!(
        body["result"]["capabilities"]["completions"].is_object(),
        "with_completion() must advertise the completions capability: {body}"
    );

    // …and the method must answer with the CompleteResult shape.
    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "completion/complete")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 31, "method": "completion/complete",
            "params": {
                "ref": { "type": "ref/prompt", "name": "example" },
                "argument": { "name": "arg", "value": "ex" },
                "_meta": meta()
            }
        }))
        .send()
        .await
        .expect("completion POST");
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.expect("json");
    assert!(
        body["result"]["completion"]["values"].is_array(),
        "CompleteResult must carry completion.values: {body}"
    );
}

// ---- completion/complete provider routing (COMP-1) ----

/// Provider serving the `name` argument of the `greet` prompt.
struct GreetNameCompleter;

impl turul_mcp_server::prelude::HasCompletionMetadata for GreetNameCompleter {
    fn method(&self) -> &str {
        "completion/complete"
    }
    fn reference(&self) -> &turul_mcp_protocol::completion::CompletionReference {
        use std::sync::OnceLock;
        use turul_mcp_protocol::completion::{CompletionReference, PromptReference};
        static REF: OnceLock<CompletionReference> = OnceLock::new();
        REF.get_or_init(|| CompletionReference::Prompt(PromptReference::new("greet")))
    }
}
impl turul_mcp_server::prelude::HasCompletionContext for GreetNameCompleter {
    fn argument(&self) -> &turul_mcp_protocol::completion::CompleteArgument {
        use std::sync::OnceLock;
        use turul_mcp_protocol::completion::CompleteArgument;
        static ARG: OnceLock<CompleteArgument> = OnceLock::new();
        ARG.get_or_init(|| CompleteArgument::new("name", ""))
    }
}
impl turul_mcp_server::prelude::HasCompletionHandling for GreetNameCompleter {}

#[async_trait::async_trait]
impl turul_mcp_server::McpCompletion for GreetNameCompleter {
    async fn complete(
        &self,
        request: turul_mcp_protocol::completion::CompleteRequest,
    ) -> McpResult<turul_mcp_protocol::completion::CompleteResult> {
        use turul_mcp_protocol::completion::{CompleteResult, CompletionResult};
        let prefix = request.params.argument.value.to_lowercase();
        let values: Vec<String> = ["alpha", "beta", "gamma"]
            .iter()
            .filter(|v| v.starts_with(&prefix))
            .map(|v| v.to_string())
            .collect();
        Ok(CompleteResult::new(CompletionResult::new(values)))
    }
}

/// Provider returning more values than the spec's 100-item response cap.
struct FloodCompleter;

impl turul_mcp_server::prelude::HasCompletionMetadata for FloodCompleter {
    fn method(&self) -> &str {
        "completion/complete"
    }
    fn reference(&self) -> &turul_mcp_protocol::completion::CompletionReference {
        use std::sync::OnceLock;
        use turul_mcp_protocol::completion::{CompletionReference, PromptReference};
        static REF: OnceLock<CompletionReference> = OnceLock::new();
        REF.get_or_init(|| CompletionReference::Prompt(PromptReference::new("flood")))
    }
}
impl turul_mcp_server::prelude::HasCompletionContext for FloodCompleter {
    fn argument(&self) -> &turul_mcp_protocol::completion::CompleteArgument {
        use std::sync::OnceLock;
        use turul_mcp_protocol::completion::CompleteArgument;
        static ARG: OnceLock<CompleteArgument> = OnceLock::new();
        ARG.get_or_init(|| CompleteArgument::new("item", ""))
    }
}
impl turul_mcp_server::prelude::HasCompletionHandling for FloodCompleter {}

#[async_trait::async_trait]
impl turul_mcp_server::McpCompletion for FloodCompleter {
    async fn complete(
        &self,
        _request: turul_mcp_protocol::completion::CompleteRequest,
    ) -> McpResult<turul_mcp_protocol::completion::CompleteResult> {
        use turul_mcp_protocol::completion::{CompleteResult, CompletionResult};
        let values: Vec<String> = (0..150).map(|i| format!("v{i:03}")).collect();
        Ok(CompleteResult::new(CompletionResult::new(values)))
    }
}

async fn start_completion_server() -> String {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let server = McpServer::builder()
        .name("completion-provider-2026-test")
        .version("0.4.0")
        .tool(EchoTool::default())
        .completion_provider(GreetNameCompleter)
        .completion_provider(FloodCompleter)
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

async fn post_completion(url: &str, prompt: &str, arg: &str, value: &str) -> serde_json::Value {
    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "completion/complete")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 40, "method": "completion/complete",
            "params": {
                "ref": { "type": "ref/prompt", "name": prompt },
                "argument": { "name": arg, "value": value },
                "_meta": meta()
            }
        }))
        .send()
        .await
        .expect("completion POST");
    assert_eq!(resp.status(), 200);
    resp.json().await.expect("json")
}

/// Registered McpCompletion providers must answer completion/complete —
/// not a hardcoded placeholder.
#[tokio::test]
async fn completion_complete_routes_to_registered_provider() {
    let url = start_completion_server().await;
    let body = post_completion(&url, "greet", "name", "a").await;
    let values = body["result"]["completion"]["values"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        values,
        vec![serde_json::json!("alpha")],
        "the provider's filtered values must reach the wire: {body}"
    );
}

/// Completion §Response: values carry "Maximum 100 items" — oversized
/// provider output is truncated with total/hasMore reflecting the cut.
#[tokio::test]
async fn completion_values_are_capped_at_100() {
    let url = start_completion_server().await;
    let body = post_completion(&url, "flood", "item", "").await;
    let completion = &body["result"]["completion"];
    assert_eq!(
        completion["values"].as_array().map(|a| a.len()),
        Some(100),
        "values must be capped at 100: {body}"
    );
    assert_eq!(completion["hasMore"], true, "{body}");
    assert_eq!(completion["total"], 150, "{body}");
}

/// Completion §Security: "Implementations MUST validate all completion
/// inputs" — malformed params are -32602, not 200-with-placeholder.
#[tokio::test]
async fn malformed_completion_params_are_rejected_with_32602() {
    let url = start_completion_server().await;
    let client = reqwest::Client::new();
    for bad_params in [
        // missing argument entirely
        serde_json::json!({ "ref": { "type": "ref/prompt", "name": "greet" }, "_meta": meta() }),
        // unknown ref type
        serde_json::json!({
            "ref": { "type": "ref/banana", "name": "greet" },
            "argument": { "name": "name", "value": "a" },
            "_meta": meta()
        }),
    ] {
        let resp = client
            .post(&url)
            .header("Accept", "application/json")
            .header("MCP-Protocol-Version", "2026-07-28")
            .header("Mcp-Method", "completion/complete")
            .json(&serde_json::json!({
                "jsonrpc": "2.0", "id": 41, "method": "completion/complete",
                "params": bad_params
            }))
            .send()
            .await
            .expect("completion POST");
        let body: serde_json::Value = resp.json().await.expect("json");
        assert_eq!(
            body["error"]["code"], -32602,
            "malformed completion params must be invalid-params: {body}"
        );
    }
}
