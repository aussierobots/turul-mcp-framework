//! Wire-level acceptance for MRTR (SEP-2322) production on the 2026 path.
//!
//! Multi Round-Trip Requests: a server needing client input during
//! `tools/call` does NOT send a server-initiated request — it returns an
//! `InputRequiredResult` (`resultType: "input_required"`, `inputRequests`,
//! opaque `requestState`), and the client retries the ORIGINAL request with
//! `inputResponses` + the echoed `requestState` under a new JSON-RPC id.
//! Servers MUST NOT request capabilities the client did not declare in the
//! request's `_meta` `clientCapabilities` (→ `-32003`, HTTP 400).
//!
//! Built only under the 2026 feature; compiles to nothing under 2025-11-25.
#![cfg(feature = "protocol-2026-07-28")]

use std::collections::HashMap;

use turul_mcp_derive::McpTool;
use turul_mcp_protocol::elicitation::{ElicitRequest, ElicitationSchema};
use turul_mcp_protocol::input_required::{InputRequest, InputRequests};
use turul_mcp_server::prelude::*;

/// Echoes only after the client answers an elicitation: first call returns
/// input-required; the retry (carrying `inputResponses`) completes.
#[derive(McpTool, Clone, Default)]
#[tool(name = "gated_echo", description = "Echo, but ask first", output = String)]
struct GatedEchoTool {}

impl GatedEchoTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        let session = session.ok_or_else(|| McpError::tool_execution("session required"))?;

        // Retry leg: the tools/call handler surfaces the retry's inputResponses.
        if let Some(responses) = session.input_responses() {
            let answer = responses
                .get("q1")
                .and_then(|r| match r {
                    turul_mcp_protocol::input_required::InputResponse::Elicit(e) => e
                        .content
                        .as_ref()
                        .and_then(|c| c.get("answer"))
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    _ => None,
                })
                .ok_or_else(|| McpError::tool_execution("q1 elicit response missing"))?;
            assert_eq!(
                session.mrtr_request_state().as_deref(),
                Some("state-1"),
                "requestState must be echoed verbatim"
            );
            return Ok(format!("answered: {answer}"));
        }

        // First leg: demand input via MRTR.
        let schema = ElicitationSchema::new().with_property(
            "answer".to_string(),
            turul_mcp_protocol::elicitation::PrimitiveSchemaDefinition::string(),
        );
        let mut requests = InputRequests::new();
        requests.insert(
            "q1".to_string(),
            InputRequest::Elicit(ElicitRequest::new_form("What is the answer?", schema)),
        );
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: Some("state-1".to_string()),
        })
    }
}

async fn start_server() -> String {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();

    let server = McpServer::builder()
        .name("mrtr-2026-test")
        .version("0.4.0")
        .tool(GatedEchoTool::default())
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

/// Per-request `_meta`; `capabilities` lets tests model what the client declared.
fn meta_with_capabilities(capabilities: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "test-client", "version": "1.0.0" },
        "io.modelcontextprotocol/clientCapabilities": capabilities
    })
}

async fn call_gated_echo(
    url: &str,
    capabilities: serde_json::Value,
    extra_params: serde_json::Value,
) -> (reqwest::StatusCode, serde_json::Value) {
    let mut params = serde_json::json!({
        "name": "gated_echo",
        "arguments": {},
        "_meta": meta_with_capabilities(capabilities)
    });
    if let (Some(p), Some(e)) = (params.as_object_mut(), extra_params.as_object()) {
        for (k, v) in e {
            p.insert(k.clone(), v.clone());
        }
    }
    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "gated_echo")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": params
        }))
        .send()
        .await
        .expect("tools/call POST");
    let status = resp.status();
    let body: serde_json::Value = resp.json().await.unwrap_or_default();
    (status, body)
}

#[tokio::test]
async fn mrtr_round_trip_completes_the_original_call() {
    let url = start_server().await;
    let elicitation_caps = serde_json::json!({ "elicitation": {} });

    // Leg 1: no inputResponses → input_required with the elicit request.
    let (status, body) =
        call_gated_echo(&url, elicitation_caps.clone(), serde_json::json!({})).await;
    assert_eq!(
        status, 200,
        "input_required is a RESULT, not an error: {body}"
    );
    let result = &body["result"];
    assert_eq!(
        result["resultType"], "input_required",
        "first leg must return InputRequiredResult: {body}"
    );
    assert_eq!(
        result["inputRequests"]["q1"]["method"], "elicitation/create",
        "the elicit request must ride inputRequests: {body}"
    );
    assert_eq!(result["requestState"], "state-1");

    // Leg 2: retry the ORIGINAL request with inputResponses + echoed state.
    let (status, body) = call_gated_echo(
        &url,
        elicitation_caps,
        serde_json::json!({
            "inputResponses": {
                "q1": { "action": "accept", "content": { "answer": "42" } }
            },
            "requestState": "state-1"
        }),
    )
    .await;
    assert_eq!(status, 200);
    let result = &body["result"];
    assert_ne!(
        result["resultType"], "input_required",
        "the retry must complete, not loop: {body}"
    );
    let text = result["content"][0]["text"].as_str().unwrap_or_default();
    assert!(
        text.contains("answered: 42"),
        "the tool must see the elicit response on the retry: {body}"
    );
}

#[tokio::test]
async fn undeclared_capability_is_rejected_with_32003() {
    let url = start_server().await;

    // The client declares NO capabilities — the server must not emit an
    // elicitation input request, per the MRTR rules: -32003 + HTTP 400.
    let (status, body) = call_gated_echo(&url, serde_json::json!({}), serde_json::json!({})).await;
    assert_eq!(
        status, 400,
        "MissingRequiredClientCapabilityError must be HTTP 400: {body}"
    );
    assert_eq!(
        body["error"]["code"], -32003,
        "must be -32003 MissingRequiredClientCapability: {body}"
    );
}

/// Keep the unused import warnings honest: HashMap is used by the tool body.
#[allow(dead_code)]
fn _t(_: HashMap<String, String>) {}
