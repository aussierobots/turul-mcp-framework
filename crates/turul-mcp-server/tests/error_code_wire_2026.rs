//! Wire-level acceptance that a permission denial and a missing resource are
//! distinguishable on 2026-07-28 — neither collapses onto the same code.
//!
//! [Error Codes](https://modelcontextprotocol.io/specification/2026-07-28/basic/index#error-codes):
//! "Implementations of this protocol version MUST NOT emit these codes:
//! ... `-32002` — resource not found (2025-11-25 and earlier; replaced by
//! `-32602`)." Before this test existed, `turul-http-mcp-server`'s
//! `error_codes::UNAUTHORIZED` was itself `-32002` — a 2026 permission denial
//! and a missing resource were wire-identical, and both were the exact code
//! this spec version forbids emitting.
//!
//! Built only under the 2026 feature; compiles to nothing under 2025-11-25.
#![cfg(feature = "protocol-2026-07-28")]

mod common;

use async_trait::async_trait;
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;
use turul_mcp_session_storage::SessionView;

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

/// Denies exactly one tool by name, unconditionally — a stand-in for a real
/// authorization check, isolating the wire shape of the rejection from any
/// particular auth scheme.
struct DenySecretTool;

#[async_trait]
impl McpMiddleware for DenySecretTool {
    async fn before_dispatch(
        &self,
        ctx: &mut RequestContext<'_>,
        _session: Option<&dyn SessionView>,
        _injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        if ctx.method() == "tools/call"
            && ctx.params().and_then(|p| p.get("name")).and_then(|v| v.as_str()) == Some("secret")
        {
            return Err(MiddlewareError::Unauthorized(
                "caller lacks permission for tool 'secret'".into(),
            ));
        }
        Ok(())
    }
}

async fn start_server() -> String {
    let reserved = common::reserve_port().await;
    let port = reserved.port;

    let server = McpServer::builder()
        .name("error-code-wire-2026-test")
        .version("0.4.0")
        .tool(EchoTool::default())
        .middleware(Arc::new(DenySecretTool))
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

async fn post_method(
    url: &str,
    rpc_method: &str,
    name_header: &str,
    params: serde_json::Value,
) -> (reqwest::StatusCode, serde_json::Value) {
    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", rpc_method)
        .header("Mcp-Name", name_header)
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": rpc_method, "params": params
        }))
        .send()
        .await
        .unwrap_or_else(|e| panic!("{rpc_method} POST failed: {e}"));
    let status = resp.status();
    let body: serde_json::Value = resp.json().await.unwrap_or_default();
    (status, body)
}

/// Same server, two requests: a middleware-rejected tool call (permission
/// denial) and a `resources/read` for a URI no resource serves (missing
/// resource). Both are well-formed requests that fail past the transport
/// layer, so both answer HTTP 200 with the error in the JSON-RPC body — the
/// two must still be distinguishable by `error.code`, and neither may be
/// `-32002`, which 2026-07-28 forbids implementations of this version from
/// emitting.
#[tokio::test]
async fn permission_denial_and_missing_resource_are_distinguishable_and_neither_is_32002() {
    let url = start_server().await;

    let (denied_status, denied_body) = post_method(
        &url,
        "tools/call",
        "secret",
        serde_json::json!({ "name": "secret", "arguments": {}, "_meta": meta() }),
    )
    .await;
    let (missing_status, missing_body) = post_method(
        &url,
        "resources/read",
        "file:///nope.txt",
        serde_json::json!({ "uri": "file:///nope.txt", "_meta": meta() }),
    )
    .await;

    assert_eq!(
        denied_status, 200,
        "a middleware rejection is a well-formed request failing past the \
         transport layer, same as a handler failure: {denied_body}"
    );
    assert_eq!(
        missing_status, 200,
        "a missing resource is a well-formed request failing inside the \
         handler: {missing_body}"
    );

    let denied_code = denied_body["error"]["code"].as_i64();
    let missing_code = missing_body["error"]["code"].as_i64();

    assert_ne!(
        denied_code, Some(-32002),
        "permission denial must not be -32002 — that code means resource-not-found \
         to every conformant 2026-07-28 peer and this spec version forbids emitting \
         it: {denied_body}"
    );
    assert_ne!(
        missing_code, Some(-32002),
        "missing resource must be -32602 on 2026-07-28, not the retired -32002: {missing_body}"
    );
    assert_eq!(missing_code, Some(-32602), "{missing_body}");
    assert_ne!(
        denied_code, missing_code,
        "a permission denial and a missing resource must carry different codes \
         so a client can tell them apart: denied={denied_body} missing={missing_body}"
    );
}
