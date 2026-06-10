//! Wire-level acceptance for SEP-2243 `Mcp-Param-*` validation (server side).
//!
//! A tool may designate parameters for header mirroring via an `x-mcp-header`
//! annotation in its `inputSchema`. A server processing the body MUST validate
//! that mirrored header values (Base64 sentinel decoded) match the body
//! arguments — missing header with the value in the body, spurious header, or
//! a decoded mismatch → HTTP 400 + JSON-RPC `-32001` (`HeaderMismatch`).
//!
//! Built only under the 2026 feature; compiles to nothing under 2025-11-25.
#![cfg(feature = "protocol-2026-07-28")]

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::{Value, json};
use turul_mcp_protocol::ToolSchema;
use turul_mcp_protocol::tools::{CallToolResult, ToolResult};
use turul_mcp_server::prelude::*;
use turul_mcp_server::{McpServer, McpTool, SessionContext};

/// Manual tool with an `x-mcp-header`-annotated `region` parameter.
struct ExecuteSqlTool {
    input_schema: ToolSchema,
}

impl ExecuteSqlTool {
    fn new() -> Self {
        let mut properties = HashMap::new();
        properties.insert(
            "region".to_string(),
            json!({ "type": "string", "x-mcp-header": "Region" }),
        );
        properties.insert("query".to_string(), json!({ "type": "string" }));
        Self {
            input_schema: ToolSchema::object()
                .with_properties(properties)
                .with_required(vec!["region".to_string(), "query".to_string()]),
        }
    }
}

impl HasBaseMetadata for ExecuteSqlTool {
    fn name(&self) -> &str {
        "execute_sql"
    }
}
impl HasDescription for ExecuteSqlTool {
    fn description(&self) -> Option<&str> {
        Some("Execute SQL in a region")
    }
}
impl HasInputSchema for ExecuteSqlTool {
    fn input_schema(&self) -> &ToolSchema {
        &self.input_schema
    }
}
impl HasOutputSchema for ExecuteSqlTool {}
impl HasAnnotations for ExecuteSqlTool {}
impl HasToolMeta for ExecuteSqlTool {}
impl HasIcons for ExecuteSqlTool {}

#[async_trait]
impl McpTool for ExecuteSqlTool {
    async fn call(
        &self,
        args: Value,
        _session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        let region = args
            .get("region")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        Ok(CallToolResult::success(vec![ToolResult::text(format!(
            "ran in {region}"
        ))]))
    }
}

async fn start_server() -> String {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();

    let server = McpServer::builder()
        .name("mcp-param-2026-test")
        .version("0.4.0")
        .tool(ExecuteSqlTool::new())
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

fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "test-client", "version": "1.0.0" },
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

async fn call_execute_sql(
    url: &str,
    region_value: &str,
    mcp_param_region: Option<&str>,
) -> (reqwest::StatusCode, Value) {
    let client = reqwest::Client::new();
    let mut req = client
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "execute_sql");
    if let Some(h) = mcp_param_region {
        req = req.header("Mcp-Param-Region", h);
    }
    let resp = req
        .json(&json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {
                "name": "execute_sql",
                "arguments": { "region": region_value, "query": "SELECT 1" },
                "_meta": meta()
            }
        }))
        .send()
        .await
        .expect("tools/call POST");
    let status = resp.status();
    let body: Value = resp.json().await.unwrap_or_default();
    (status, body)
}

#[tokio::test]
async fn matching_param_header_passes_validation() {
    let url = start_server().await;
    let (status, body) = call_execute_sql(&url, "us-west1", Some("us-west1")).await;
    assert_eq!(status, 200, "matching header/body must pass: {body}");
    let text = body["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default();
    assert!(text.contains("ran in us-west1"), "{body}");
}

#[tokio::test]
async fn base64_encoded_param_header_decodes_and_matches() {
    let url = start_server().await;
    // " padded " encodes to the spec's example sentinel form.
    let (status, body) = call_execute_sql(&url, " padded ", Some("=?base64?IHBhZGRlZCA=?=")).await;
    assert_eq!(
        status, 200,
        "Base64 sentinel values must be decoded before comparison: {body}"
    );
}

#[tokio::test]
async fn omitted_param_header_with_body_value_is_rejected() {
    let url = start_server().await;
    let (status, body) = call_execute_sql(&url, "us-west1", None).await;
    assert_eq!(
        status, 400,
        "annotated param in body without its header must be rejected: {body}"
    );
    assert_eq!(body["error"]["code"], -32001, "{body}");
}

#[tokio::test]
async fn mismatched_param_header_is_rejected() {
    let url = start_server().await;
    let (status, body) = call_execute_sql(&url, "us-west1", Some("eu-central1")).await;
    assert_eq!(status, 400, "header/body mismatch must be rejected: {body}");
    assert_eq!(body["error"]["code"], -32001, "{body}");
}
