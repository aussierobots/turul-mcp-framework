//! Over real HTTP on 2026-07-28: a derive-generated tool refuses a bad argument as a
//! `result` carrying `isError: true` (schema.ts `CallToolResult.isError`; server/tools
//! § Error Handling, "Input validation errors"), not as a JSON-RPC `error`. Protocol
//! errors stay protocol errors: an unknown tool and a request that fails the
//! `CallToolRequest` schema are `-32602`.
#![cfg(feature = "protocol-2026-07-28")]

mod common;

use serde_json::{Value, json};
use turul_mcp_derive::{McpTool, mcp_tool};
use turul_mcp_server::prelude::*;

#[derive(McpTool, Clone, Default)]
#[tool(name = "wire_derive", description = "Derive tool", output = String)]
struct WireDerive {
    #[param(description = "Required")]
    a: String,
    #[param(description = "Optional")]
    limit: Option<u32>,
}

impl WireDerive {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok(format!("{}:{:?}", self.a, self.limit))
    }
}

#[mcp_tool(name = "wire_attr", description = "Attribute tool")]
async fn wire_attr(a: String, limit: Option<u32>) -> McpResult<String> {
    Ok(format!("{a}:{limit:?}"))
}

struct Wire {
    url: String,
    client: reqwest::Client,
}

async fn start() -> Wire {
    let reserved = common::reserve_port().await;
    let port = reserved.port;
    let server = McpServer::builder()
        .name("tool-argument-errors-2026")
        .version("0.4.0")
        .tool(WireDerive::default())
        .tool_fn(wire_attr)
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
    drop(reserved);
    Wire { url, client }
}

impl Wire {
    async fn rpc(&self, method: &str, name: Option<&str>, params: Value) -> Value {
        let mut params = params;
        params["_meta"] = json!({
            "io.modelcontextprotocol/protocolVersion": "2026-07-28",
            "io.modelcontextprotocol/clientCapabilities": {}
        });
        let mut req = self
            .client
            .post(&self.url)
            .header("Accept", "application/json")
            .header("MCP-Protocol-Version", "2026-07-28")
            .header("Mcp-Method", method);
        if let Some(name) = name {
            req = req.header("Mcp-Name", name);
        }
        req.json(&json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}))
            .send()
            .await
            .expect("POST")
            .json()
            .await
            .expect("json body")
    }

    async fn call(&self, tool: &str, arguments: Value) -> Value {
        self.rpc(
            "tools/call",
            Some(tool),
            json!({"name": tool, "arguments": arguments}),
        )
        .await
    }
}

fn assert_tool_error_naming(body: &Value, keys: &[&str]) {
    assert!(
        body.get("error").is_none(),
        "must be a result, not a JSON-RPC error: {body}"
    );
    assert_eq!(body["result"]["isError"], json!(true), "{body}");
    assert_eq!(body["result"]["resultType"], json!("complete"), "{body}");
    let text = body["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default();
    for key in keys {
        assert!(text.contains(key), "text must name `{key}`: {body}");
    }
}

#[tokio::test]
async fn tools_list_advertises_a_closed_input_schema() {
    let wire = start().await;
    let body = wire.rpc("tools/list", None, json!({})).await;
    for name in ["wire_derive", "wire_attr"] {
        let tool = body["result"]["tools"]
            .as_array()
            .unwrap_or_else(|| panic!("no tools: {body}"))
            .iter()
            .find(|t| t["name"] == name)
            .unwrap_or_else(|| panic!("{name} absent: {body}"));
        assert_eq!(
            tool["inputSchema"]["additionalProperties"],
            json!(false),
            "{tool}"
        );
    }
}

#[tokio::test]
async fn undeclared_argument_is_a_tool_error_result_on_the_wire() {
    let wire = start().await;
    for tool in ["wire_derive", "wire_attr"] {
        let body = wire.call(tool, json!({"a": "x", "bogus": 1})).await;
        assert_tool_error_naming(&body, &["bogus"]);
    }
}

#[tokio::test]
async fn wrong_typed_optional_argument_is_a_tool_error_result_on_the_wire() {
    let wire = start().await;
    for tool in ["wire_derive", "wire_attr"] {
        let body = wire.call(tool, json!({"a": "x", "limit": "5"})).await;
        assert_tool_error_naming(&body, &["limit"]);
    }
}

#[tokio::test]
async fn missing_required_argument_is_a_tool_error_result_on_the_wire() {
    let wire = start().await;
    for tool in ["wire_derive", "wire_attr"] {
        assert_tool_error_naming(&wire.call(tool, json!({})).await, &["a"]);
    }
}

#[tokio::test]
async fn declared_only_call_succeeds_on_the_wire() {
    let wire = start().await;
    for tool in ["wire_derive", "wire_attr"] {
        let body = wire.call(tool, json!({"a": "x", "limit": 5})).await;
        assert!(body.get("error").is_none(), "{body}");
        assert_ne!(body["result"]["isError"], json!(true), "{body}");
    }
}

#[tokio::test]
async fn unknown_tool_is_still_a_protocol_error() {
    let wire = start().await;
    let body = wire.call("no_such_tool", json!({})).await;
    assert_eq!(body["error"]["code"], json!(-32602), "{body}");
}

#[tokio::test]
async fn non_object_arguments_are_a_protocol_error() {
    let wire = start().await;
    let body = wire.call("wire_derive", json!("not-an-object")).await;
    assert_eq!(body["error"]["code"], json!(-32602), "{body}");
}
