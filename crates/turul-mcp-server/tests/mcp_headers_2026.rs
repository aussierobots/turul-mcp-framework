//! Wire-level acceptance for the 2026-07-28 request-metadata headers.
//!
//! Streamable HTTP §Request Metadata / §Server Validation:
//!   - Every POST MUST carry `MCP-Protocol-Version`; a server that does not
//!     support pre-2025-06-18 clients MUST reject requests without it.
//!   - `Mcp-Method` is REQUIRED on all requests and notifications and MUST
//!     match the body `method`.
//!   - `Mcp-Name` is REQUIRED for `tools/call` (`params.name`),
//!     `resources/read` (`params.uri`), and `prompts/get` (`params.name`)
//!     and MUST match the body value.
//!   - Validation failures → HTTP 400 + JSON-RPC `-32020` (`HeaderMismatch`).
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

async fn start_server() -> String {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();

    let server = McpServer::builder()
        .name("headers-2026-test")
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

fn tools_call_body() -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": { "name": "echo", "arguments": { "message": "hi" }, "_meta": meta() }
    })
}

/// Same shape as `tools_call_body()` but with an arbitrary `params.name` — the
/// tool need not exist to test the header-validation layer, which runs before
/// dispatch.
fn tools_call_body_with_name(name: &str) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": { "name": name, "arguments": { "message": "hi" }, "_meta": meta() }
    })
}

async fn assert_header_mismatch(resp: reqwest::Response, case: &str) {
    assert_eq!(resp.status(), 400, "{case}: must be HTTP 400 Bad Request");
    let body: serde_json::Value = resp.json().await.expect("error body");
    assert_eq!(
        body["error"]["code"], -32020,
        "{case}: must be JSON-RPC -32020 HeaderMismatch, got: {body}"
    );
}

#[tokio::test]
async fn fully_headed_request_succeeds() {
    let url = start_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "echo")
        .json(&tools_call_body())
        .send()
        .await
        .expect("headed tools/call");
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.expect("body");
    assert!(body["result"].is_object(), "expected a result, got {body}");
}

#[tokio::test]
async fn missing_protocol_version_header_is_rejected() {
    let url = start_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "echo")
        .json(&tools_call_body())
        .send()
        .await
        .expect("no MCP-Protocol-Version");
    assert_header_mismatch(resp, "missing MCP-Protocol-Version").await;
}

#[tokio::test]
async fn missing_mcp_method_header_is_rejected() {
    let url = start_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Name", "echo")
        .json(&tools_call_body())
        .send()
        .await
        .expect("no Mcp-Method");
    assert_header_mismatch(resp, "missing Mcp-Method").await;
}

#[tokio::test]
async fn mismatched_mcp_method_header_is_rejected() {
    let url = start_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/list")
        .header("Mcp-Name", "echo")
        .json(&tools_call_body())
        .send()
        .await
        .expect("mismatched Mcp-Method");
    assert_header_mismatch(resp, "Mcp-Method header/body mismatch").await;
}

#[tokio::test]
async fn missing_mcp_name_on_tools_call_is_rejected() {
    let url = start_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .json(&tools_call_body())
        .send()
        .await
        .expect("no Mcp-Name");
    assert_header_mismatch(resp, "missing Mcp-Name on tools/call").await;
}

#[tokio::test]
async fn mismatched_mcp_name_is_rejected() {
    let url = start_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "not-echo")
        .json(&tools_call_body())
        .send()
        .await
        .expect("mismatched Mcp-Name");
    assert_header_mismatch(resp, "Mcp-Name header/body mismatch").await;
}

/// SEP-2243 §Value Encoding, extended to `Mcp-Name` by upstream commit
/// `71d924e2`: servers MUST decode a Base64-sentinel-encoded `Mcp-Name`
/// before comparing it to the body value. `=?base64?IHBhZGRlZCA=?=` is the
/// spec's own encoding example for `" padded "`.
#[tokio::test]
async fn base64_encoded_mcp_name_decodes_and_matches() {
    let url = start_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "=?base64?IHBhZGRlZCA=?=")
        .json(&tools_call_body_with_name(" padded "))
        .send()
        .await
        .expect("Base64-sentinel Mcp-Name");
    assert_ne!(
        resp.status(),
        400,
        "a correctly-encoded Mcp-Name matching the body value must pass header validation"
    );
}

/// An encoded `Mcp-Name` that decodes to a value different from the body
/// must still be rejected as a HeaderMismatch — decoding must not weaken the
/// comparison into an always-pass.
#[tokio::test]
async fn base64_encoded_mcp_name_mismatch_is_rejected() {
    let url = start_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        // Decodes to "Hello, 世界" — does not match the body's " padded ".
        .header("Mcp-Name", "=?base64?SGVsbG8sIOS4lueVjA==?=")
        .json(&tools_call_body_with_name(" padded "))
        .send()
        .await
        .expect("mismatched Base64-encoded Mcp-Name");
    assert_header_mismatch(resp, "decoded Mcp-Name does not match body value").await;
}

#[tokio::test]
async fn resources_read_requires_uri_as_mcp_name() {
    let url = start_server().await;
    let client = reqwest::Client::new();

    // Correct: Mcp-Name mirrors params.uri. The resource itself need not exist
    // (that's a -32602 at dispatch, not a header failure) — but the header
    // layer must accept the matching pair...
    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "resources/read")
        .header("Mcp-Name", "file:///missing.txt")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "resources/read",
            "params": { "uri": "file:///missing.txt", "_meta": meta() }
        }))
        .send()
        .await
        .expect("headed resources/read");
    assert_ne!(
        resp.status(),
        400,
        "matching Mcp-Name/uri must pass header validation"
    );

    // ...and reject a mismatched pair.
    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "resources/read")
        .header("Mcp-Name", "file:///other.txt")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 3, "method": "resources/read",
            "params": { "uri": "file:///missing.txt", "_meta": meta() }
        }))
        .send()
        .await
        .expect("mismatched resources/read");
    assert_header_mismatch(resp, "Mcp-Name/uri mismatch on resources/read").await;
}

#[tokio::test]
async fn methods_without_name_need_no_mcp_name() {
    let url = start_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/list")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 4, "method": "tools/list",
            "params": { "_meta": meta() }
        }))
        .send()
        .await
        .expect("tools/list without Mcp-Name");
    assert_eq!(resp.status(), 200, "tools/list requires no Mcp-Name header");
}

#[tokio::test]
async fn notifications_also_require_mcp_method() {
    let url = start_server().await;
    let client = reqwest::Client::new();

    // With the header → 202 Accepted.
    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "notifications/progress")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "method": "notifications/progress",
            "params": { "progressToken": "t1", "progress": 0.5 }
        }))
        .send()
        .await
        .expect("headed notification");
    assert_eq!(resp.status(), 202);

    // Without it → 400 (notifications can't carry a JSON-RPC id; the error
    // body MAY be an id-less error response, so only the status is asserted).
    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "method": "notifications/progress",
            "params": { "progressToken": "t1", "progress": 0.5 }
        }))
        .send()
        .await
        .expect("headerless notification");
    assert_eq!(
        resp.status(),
        400,
        "notifications without Mcp-Method must be rejected"
    );
}

/// Versioning §Backward Compatibility: "A server that supports only modern
/// versions SHOULD name the protocol versions it supports in any error it
/// returns to an initialize request, on any transport." A true legacy client
/// sends `initialize` with NO version header — the missing-header rejection
/// may be its only diagnostic, so it must carry `error.data.supported`.
#[tokio::test]
async fn headerless_initialize_rejection_names_supported_versions() {
    let url = start_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": { "name": "legacy", "version": "1.0" }
            }
        }))
        .send()
        .await
        .expect("headerless initialize");
    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.expect("body");
    assert_eq!(body["error"]["code"], -32020);
    assert_eq!(
        body["error"]["data"]["supported"],
        serde_json::json!(["2026-07-28"]),
        "the initialize rejection must name the supported versions: {body}"
    );
}
