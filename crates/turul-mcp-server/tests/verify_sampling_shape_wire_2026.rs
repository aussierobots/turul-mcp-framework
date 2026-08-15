//! Sampling message-shape MUSTs on `sampling/createMessage` input requests,
//! observed on the wire.
//!
//! Drives a real HTTP server (reqwest -> streamable handler -> tools/call
//! dispatch -> `input_required_to_result`). `sampling_shape_2026.rs` covers an
//! assistant ToolUse with no following message; the bad shape here is the other
//! message-shape MUST — a user message that mixes a text block with a
//! ToolResult block. Also asserts the JSON-RPC response `id` echoes the request
//! id for both numeric and string ids.
#![cfg(feature = "protocol-2026-07-28")]

mod common;

use turul_mcp_derive::McpTool;
use turul_mcp_protocol::input_required::{InputRequest, InputRequests};
use turul_mcp_server::prelude::*;

/// Emits a CreateMessage whose USER message mixes text + ToolResult — invalid:
/// a user message containing a ToolResult MUST contain ONLY ToolResult blocks.
#[derive(McpTool, Clone, Default)]
#[tool(name = "mixed_user_msg", description = "user text mixed with tool_result", output = String)]
struct MixedUserMsgTool {}

impl MixedUserMsgTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        #[allow(deprecated)]
        let request = {
            use turul_mcp_protocol::sampling::{
                CreateMessageRequest, Role, SamplingMessage, SamplingMessageContent,
                SamplingMessageContentBlock,
            };
            let text = SamplingMessageContentBlock::text("here is a result");
            let tool_result = SamplingMessageContentBlock::ToolResult {
                tool_use_id: "call-1".to_string(),
                content: vec![],
                structured_content: None,
                is_error: None,
                meta: None,
            };
            CreateMessageRequest::new(
                vec![SamplingMessage::new(
                    Role::User,
                    SamplingMessageContent::Multiple(vec![text, tool_result]),
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

async fn start_server() -> String {
    let reserved = common::reserve_port().await;
    let port = reserved.port;
    let server = McpServer::builder()
        .name("sampling-shape-wire-2026")
        .version("0.4.0")
        .tool(MixedUserMsgTool::default())
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

async fn post_call(url: &str, id: serde_json::Value) -> (reqwest::StatusCode, serde_json::Value) {
    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "mixed_user_msg")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": {
                "name": "mixed_user_msg",
                "arguments": {},
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                    "io.modelcontextprotocol/clientInfo": { "name": "verify", "version": "1.0.0" },
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
async fn mixed_user_message_rejected_200_neg32602_id_echoed_numeric() {
    let url = start_server().await;
    let (status, body) = post_call(&url, serde_json::json!(4242)).await;
    assert_eq!(
        status, 200,
        "generic JSON-RPC error must ride HTTP 200, not 400: {body}"
    );
    assert_eq!(
        body["error"]["code"], -32602,
        "must be InvalidParams -32602: {body}"
    );
    assert_eq!(
        body["id"],
        serde_json::json!(4242),
        "response id must echo request id: {body}"
    );
    // The domain error must actually be the shape rejection, not something else.
    let m = body["error"]["message"].as_str().unwrap_or("");
    assert!(
        m.contains("message shape") || m.contains("ToolResult"),
        "error must be the sampling shape rejection: {body}"
    );
}

#[tokio::test]
async fn string_id_is_echoed() {
    let url = start_server().await;
    let (status, body) = post_call(&url, serde_json::json!("req-abc")).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["error"]["code"], -32602, "{body}");
    assert_eq!(
        body["id"],
        serde_json::json!("req-abc"),
        "string id must echo: {body}"
    );
}
