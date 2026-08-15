//! Wire-level acceptance for the sampling message-shape MUSTs: a
//! `sampling/createMessage` input request produced by a tool (MRTR, SEP-2322)
//! must satisfy them before the server packages it into an
//! `InputRequiredResult` — an assistant `ToolUse` block not immediately
//! followed by a matching `ToolResult`-only user message is rejected with
//! `-32602 InvalidParams` (HTTP 200), not silently forwarded to the client.
//!
//! Built only under the 2026 feature; compiles to nothing under 2025-11-25.
#![cfg(feature = "protocol-2026-07-28")]

mod common;

use turul_mcp_derive::McpTool;
use turul_mcp_protocol::input_required::{InputRequest, InputRequests};
use turul_mcp_server::prelude::*;

/// Requests sampling with an invalid message shape: an assistant `ToolUse`
/// block with no following `ToolResult` message at all.
#[derive(McpTool, Clone, Default)]
#[tool(name = "bad_shape_sampler", description = "Sampling with an invalid message shape", output = String)]
struct BadShapeSamplerTool {}

impl BadShapeSamplerTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        #[allow(deprecated)]
        let request = {
            use turul_mcp_protocol::sampling::{
                CreateMessageRequest, Role, SamplingMessage, SamplingMessageContent,
                SamplingMessageContentBlock,
            };
            let tool_use = SamplingMessageContentBlock::ToolUse {
                id: "call-1".to_string(),
                name: "get_weather".to_string(),
                input: std::collections::HashMap::new(),
                meta: None,
            };
            CreateMessageRequest::new(
                vec![SamplingMessage::new(
                    Role::Assistant,
                    SamplingMessageContent::Single(tool_use),
                )],
                32,
            )
        };
        let mut requests = InputRequests::new();
        requests.insert("s1".to_string(), InputRequest::CreateMessage(request));
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: None,
        })
    }
}

/// Requests well-formed sampling: a plain user text message, no tool blocks.
#[derive(McpTool, Clone, Default)]
#[tool(name = "good_shape_sampler", description = "Sampling with a valid message shape", output = String)]
struct GoodShapeSamplerTool {}

impl GoodShapeSamplerTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        #[allow(deprecated)]
        let request = {
            use turul_mcp_protocol::sampling::{CreateMessageRequest, SamplingMessage};
            CreateMessageRequest::new(vec![SamplingMessage::user_text("hi")], 32)
        };
        let mut requests = InputRequests::new();
        requests.insert("s1".to_string(), InputRequest::CreateMessage(request));
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: None,
        })
    }
}

async fn start_server() -> String {
    let reserved = common::reserve_port().await;
    let port = reserved.port;
    let server = McpServer::builder()
        .name("sampling-shape-2026")
        .version("0.4.0")
        .tool(BadShapeSamplerTool::default())
        .tool(GoodShapeSamplerTool::default())
        .bind_address(format!("127.0.0.1:{port}").parse().unwrap())
        .build()
        .expect("build 2026 server");
    tokio::spawn(async move {
        server.run().await.ok();
    });
    let url = format!("http://127.0.0.1:{port}/mcp");
    let probe = reqwest::Client::new();
    for _ in 0..50 {
        if probe.get(&url).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    url
}

async fn call_tool(url: &str, tool: &str) -> (reqwest::StatusCode, serde_json::Value) {
    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", tool)
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {
                "name": tool,
                "arguments": {},
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                    "io.modelcontextprotocol/clientInfo": { "name": "test-client", "version": "1.0.0" },
                    "io.modelcontextprotocol/clientCapabilities": { "sampling": {} }
                }
            }
        }))
        .send()
        .await
        .expect("tools/call POST");
    let status = resp.status();
    let body: serde_json::Value = resp.json().await.unwrap_or_default();
    (status, body)
}

#[tokio::test]
async fn invalid_sampling_message_shape_is_rejected_with_invalid_params() {
    let url = start_server().await;
    let (status, body) = call_tool(&url, "bad_shape_sampler").await;
    // Generic JSON-RPC errors (unlike -32021/-32020, which the transport
    // special-cases to HTTP 400) ride ordinary HTTP 200 with the error in
    // the body — the same contract as any other tools/call failure.
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["error"]["code"], -32602, "{body}");
}

#[tokio::test]
async fn valid_sampling_message_shape_reaches_input_required() {
    let url = start_server().await;
    let (status, body) = call_tool(&url, "good_shape_sampler").await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["result"]["resultType"], "input_required", "{body}");
}
