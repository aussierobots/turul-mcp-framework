//! Wire-level acceptance for how a 2026-07-28 failure reaches the client:
//! which HTTP status carries it, and whether it is a protocol error or a
//! tool-reported one.
//!
//! Streamable HTTP §Protocol Version Header: "If the server does not implement
//! the requested RPC method, it MUST respond with `404 Not Found` and a
//! JSON-RPC error with code `-32601` (Method not found). The JSON-RPC error
//! body distinguishes this case from a 404 returned by a legacy HTTP+SSE
//! server that does not host the modern MCP endpoint."
//!
//! Methods absent from the 2026-07-28 schema (`ping`, `initialize`, `tasks/*`,
//! `logging/setLevel`, `resources/subscribe`) are unknown methods here.
//!
//! Status is layered, and the split is the contract: the transport rejects
//! malformed traffic with 4xx and an unimplemented method with 404, while a
//! well-formed request that fails *inside* a handler is answered 200 with the
//! error in the JSON-RPC body. Server §Tools adds a third outcome that is not
//! an error at all at the protocol layer — a tool reporting its own failure via
//! `isError: true`, "so that the LLM can see that an error occurred".
//!
//! Built only under the 2026 feature; compiles to nothing under 2025-11-25.
#![cfg(feature = "protocol-2026-07-28")]

mod common;

use std::collections::HashMap;

use serde_json::Value;
use turul_mcp_derive::McpTool;
use turul_mcp_protocol::tools::{CallToolResult, ToolAnnotations, ToolResult, ToolSchema};
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

/// Reports a domain failure the way Server §Tools prescribes: a *successful*
/// JSON-RPC result carrying `isError: true`, not a JSON-RPC error object.
#[derive(Clone)]
struct FailingTool;

impl HasBaseMetadata for FailingTool {
    fn name(&self) -> &str {
        "always_fails"
    }
}
impl HasDescription for FailingTool {
    fn description(&self) -> Option<&str> {
        Some("Always reports a domain failure")
    }
}
impl HasInputSchema for FailingTool {
    fn input_schema(&self) -> &ToolSchema {
        static SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        SCHEMA.get_or_init(ToolSchema::object)
    }
}
impl HasOutputSchema for FailingTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}
impl HasAnnotations for FailingTool {
    fn annotations(&self) -> Option<&ToolAnnotations> {
        None
    }
}
impl HasToolMeta for FailingTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}
impl HasIcons for FailingTool {}
impl HasExecution for FailingTool {}

#[async_trait::async_trait]
impl McpTool for FailingTool {
    async fn call(
        &self,
        _args: Value,
        _session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        Ok(CallToolResult::error(vec![ToolResult::text(
            "upstream inventory service returned 503",
        )]))
    }
}

async fn start_server() -> String {
    let reserved = common::reserve_port().await;
    let port = reserved.port;

    let server = McpServer::builder()
        .name("errmap-2026-test")
        .version("0.4.0")
        .tool(EchoTool::default())
        .tool(FailingTool)
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

/// POST a fully-headed request for `rpc_method` and return (status, body).
async fn post_method(url: &str, rpc_method: &str) -> (reqwest::StatusCode, serde_json::Value) {
    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", rpc_method)
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": rpc_method,
            "params": { "_meta": meta() }
        }))
        .send()
        .await
        .unwrap_or_else(|e| panic!("{rpc_method} POST failed: {e}"));
    let status = resp.status();
    let body: serde_json::Value = resp.json().await.unwrap_or_default();
    (status, body)
}

/// POST a removed method as a real JSON-RPC *notification* — no `id`. This is
/// the envelope a client actually sends for a `notifications/*` method; the
/// id-carrying form in [`post_method`] exercises the request path instead.
async fn post_notification(url: &str, rpc_method: &str) -> reqwest::StatusCode {
    let client = reqwest::Client::new();
    client
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", rpc_method)
        .json(&serde_json::json!({ "jsonrpc": "2.0", "method": rpc_method }))
        .send()
        .await
        .unwrap_or_else(|e| panic!("{rpc_method} notification POST failed: {e}"))
        .status()
}

/// POST a `tools/call` for `tool_name`, carrying the `Mcp-Name` header the
/// 2026 core requires to agree with `params.name`.
async fn call_tool(
    url: &str,
    tool_name: &str,
    header_name: &str,
) -> (reqwest::StatusCode, serde_json::Value) {
    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", header_name)
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 7, "method": "tools/call",
            "params": { "name": tool_name, "arguments": {}, "_meta": meta() }
        }))
        .send()
        .await
        .expect("tools/call POST");
    let status = resp.status();
    let body: serde_json::Value = resp.json().await.unwrap_or_default();
    (status, body)
}

/// Server §Tools: a tool's own failure is "reported within the result object"
/// with `isError: true` — NOT a JSON-RPC error. The two are different outcomes
/// for a client: the first is content to hand back to the model, the second is
/// a call that never ran. Asserted here against the same server, back to back,
/// so the distinction is pinned rather than each half read in isolation.
#[tokio::test]
async fn tool_domain_failure_is_is_error_not_a_json_rpc_error() {
    let url = start_server().await;

    let (status, body) = call_tool(&url, "always_fails", "always_fails").await;
    assert_eq!(
        status, 200,
        "a tool-reported failure is a successful RPC: {body}"
    );
    assert!(
        body.get("error").is_none(),
        "a tool-reported failure must not become a JSON-RPC error: {body}"
    );
    assert_eq!(
        body["result"]["isError"], true,
        "the failure must be flagged with isError: true: {body}"
    );
    assert!(
        body["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .contains("503"),
        "the failure description must reach the model as content: {body}"
    );

    // The contrast: a call that never reached a tool IS a JSON-RPC error, and
    // carries no result to flag.
    let (_, body) = call_tool(&url, "no_such_tool", "no_such_tool").await;
    assert_eq!(
        body["error"]["code"], -32602,
        "an unknown tool is a protocol error, not a tool-reported one: {body}"
    );
    assert!(
        body.get("result").is_none(),
        "a protocol error carries no result to flag isError on: {body}"
    );
}

/// The status layering: a well-formed request whose failure happens inside a
/// handler is answered HTTP 200 with the error in the JSON-RPC body — the
/// opposite half of the 4xx/404 cases above. A client that routes on HTTP
/// status alone would mistake one for the other.
#[tokio::test]
async fn handler_level_failure_is_http_200_with_the_error_in_the_body() {
    let url = start_server().await;

    let (status, body) = call_tool(&url, "no_such_tool", "no_such_tool").await;
    assert_eq!(
        status, 200,
        "a well-formed request that fails inside the handler is 200, not 4xx: {body}"
    );
    assert_eq!(body["error"]["code"], -32602, "{body}");
    assert_eq!(
        body["id"], 7,
        "the error must still echo the request id: {body}"
    );

    // Same server, same shape of failure, different layer: an unimplemented
    // method never reaches a handler and is 404 instead.
    let (status, body) = post_method(&url, "frobnicate/run").await;
    assert_eq!(
        status, 404,
        "an unimplemented method is rejected before dispatch: {body}"
    );
}

#[tokio::test]
async fn unknown_method_gets_http_404_with_method_not_found() {
    let url = start_server().await;
    let (status, body) = post_method(&url, "frobnicate/run").await;
    assert_eq!(status, 404, "unknown method must be HTTP 404, got: {body}");
    assert_eq!(
        body["error"]["code"], -32601,
        "404 body must carry JSON-RPC -32601 so clients can distinguish it \
         from a legacy HTTP+SSE server's 404: {body}"
    );
}

#[tokio::test]
async fn methods_absent_from_the_2026_schema_get_404() {
    let url = start_server().await;
    // ping/initialize/tasks/logging-setLevel/resources-subscribe have no
    // bindings in the pinned 2026-07-28 schema — a 2026-only server does not
    // implement them.
    // roots/list is server→client only on 2026 (it rides MRTR input
    // requests); a stateless server hosting it inbound is non-spec, and
    // notifications/roots/list_changed has no binding in the pinned schema.
    for method in [
        "ping",
        "initialize",
        "tasks/get",
        "tasks/list",
        "logging/setLevel",
        "resources/subscribe",
        "roots/list",
        // Sent here as id-carrying *requests*, which is the wrong envelope for a
        // notification method — the notification path is covered separately by
        // `removed_notification_methods_are_acked_not_dispatched`.
        "notifications/roots/list_changed",
        "notifications/roots/listChanged",
    ] {
        let (status, body) = post_method(&url, method).await;
        assert_eq!(
            status, 404,
            "{method} is not a 2026-07-28 method — must be HTTP 404, got: {body}"
        );
        assert_eq!(
            body["error"]["code"], -32601,
            "{method}: 404 body must carry -32601: {body}"
        );
    }
}

#[tokio::test]
async fn known_methods_are_unaffected() {
    let url = start_server().await;
    let (status, body) = post_method(&url, "tools/list").await;
    assert_eq!(status, 200);
    assert!(body["result"].is_object(), "tools/list result: {body}");

    let (status, body) = post_method(&url, "server/discover").await;
    assert_eq!(status, 200);
    assert!(body["result"].is_object(), "server/discover result: {body}");
}

/// Removed *notification* methods, sent in the envelope a real client uses (no
/// `id`), are acknowledged rather than routed anywhere. JSON-RPC notifications
/// never carry a response, so 202 is the deliberate posture for an unrecognised
/// one — the contract being pinned here is that none of them revives the
/// 2025-11-25 lifecycle: `notifications/initialized` no longer takes the
/// synchronous is-initialized path on a 2026-07-28 build, and a subsequent
/// request is unaffected by having sent it.
#[tokio::test]
async fn removed_notification_methods_are_acked_not_dispatched() {
    let url = start_server().await;

    for method in [
        "notifications/initialized",
        "notifications/roots/list_changed",
        "notifications/roots/listChanged",
    ] {
        let status = post_notification(&url, method).await;
        assert_eq!(
            status, 202,
            "{method} is a notification: it must be acked with 202, not answered"
        );
    }

    // The removed lifecycle notifications left no state behind — a normal
    // request still succeeds and is not gated on any initialization flag.
    let (status, body) = post_method(&url, "tools/list").await;
    assert_eq!(
        status, 200,
        "tools/list after removed notifications: {body}"
    );
    assert_eq!(
        body["result"]["resultType"], "complete",
        "tools/list must still complete normally: {body}"
    );
    assert!(
        body["error"].is_null(),
        "no error expected after removed notifications: {body}"
    );
}
