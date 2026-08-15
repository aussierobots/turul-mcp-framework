//! End-to-end schema fidelity on the 2026 path: what a derived tool declares
//! must reach `tools/list` without downgrades, and `tools/call`'s
//! `structuredContent` must match the advertised `outputSchema` shape.
//!
//! Pins the lossless pipeline at the wire: 2020-12 compositions
//! (`oneOf`/`const`) and `$defs`-referenced subschemas must arrive intact,
//! with no dangling `$ref`.
//!
//! Built only under the 2026 feature; compiles to nothing under 2025-11-25.
#![cfg(feature = "protocol-2026-07-28")]

mod common;

use serde::{Deserialize, Serialize};
use turul_mcp_builders::schemars;
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;

/// Data-bearing tagged union — renders as `oneOf` with `const` tags, which
/// the structured schema model cannot express.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, Default)]
#[serde(tag = "kind")]
enum Shape {
    #[default]
    Point,
    Circle {
        radius: f64,
    },
    Rect {
        w: f64,
        h: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct RenderResult {
    area: f64,
    label: String,
}

#[derive(McpTool, Clone, Default)]
#[tool(name = "render", description = "Render a shape", output = RenderResult)]
struct RenderTool {
    #[param(description = "Shape to render")]
    shape: Shape,
    #[param(description = "Drawing title")]
    title: String,
}

impl RenderTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<RenderResult> {
        let area = match &self.shape {
            Shape::Point => 0.0,
            Shape::Circle { radius } => std::f64::consts::PI * radius * radius,
            Shape::Rect { w, h } => w * h,
        };
        Ok(RenderResult {
            area,
            label: self.title.clone(),
        })
    }
}

async fn start_server() -> String {
    let reserved = common::reserve_port().await;
    let port = reserved.port;

    let server = McpServer::builder()
        .name("schema-fidelity-2026")
        .version("0.4.0")
        .tool(RenderTool::default())
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

async fn post(
    url: &str,
    rpc_method: &str,
    name_header: Option<&str>,
    params: serde_json::Value,
) -> serde_json::Value {
    let client = reqwest::Client::new();
    let mut req = client
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", rpc_method);
    if let Some(n) = name_header {
        req = req.header("Mcp-Name", n);
    }
    let resp = req
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": rpc_method, "params": params
        }))
        .send()
        .await
        .expect("POST");
    assert_eq!(resp.status(), 200);
    resp.json().await.expect("json body")
}

#[tokio::test]
async fn tools_list_carries_the_full_2020_12_schema() {
    let url = start_server().await;
    let body = post(
        &url,
        "tools/list",
        None,
        serde_json::json!({"_meta": meta()}),
    )
    .await;
    let tool = body["result"]["tools"]
        .as_array()
        .and_then(|t| t.iter().find(|t| t["name"] == "render"))
        .expect("render tool listed")
        .clone();

    // Input: the tagged union survives as oneOf/anyOf with both variants and
    // their payload properties; nothing dangles.
    let shape = tool["inputSchema"]["properties"]["shape"].to_string();
    assert!(
        shape.contains("oneOf") || shape.contains("anyOf"),
        "tagged-union composition must reach tools/list: {tool}"
    );
    assert!(
        shape.contains("Circle") && shape.contains("Rect") && shape.contains("radius"),
        "variant tags and payloads must reach tools/list: {tool}"
    );
    let input = tool["inputSchema"].to_string();
    assert!(
        !input.contains("\"$ref\""),
        "no dangling $ref may reach the wire: {tool}"
    );

    // Output: the schemars-detected output schema is advertised with the
    // nested field shape intact, under the derive-chosen wrapper field.
    let output = &tool["outputSchema"];
    assert!(
        output.is_object(),
        "outputSchema must be advertised: {tool}"
    );
    let field = output["required"][0]
        .as_str()
        .expect("outputSchema declares its wrapper field as required");
    let result_schema = &output["properties"][field];
    assert_eq!(
        result_schema["properties"]["area"]["type"], "number",
        "{tool}"
    );
    assert_eq!(
        result_schema["properties"]["label"]["type"], "string",
        "{tool}"
    );
    let required: Vec<_> = result_schema["required"]
        .as_array()
        .map(|a| a.iter().filter_map(|r| r.as_str()).collect())
        .unwrap_or_default();
    assert!(
        required.contains(&"area") && required.contains(&"label"),
        "output required fields must survive: {tool}"
    );
}

#[tokio::test]
async fn structured_content_matches_the_advertised_output_schema() {
    let url = start_server().await;
    let body = post(
        &url,
        "tools/call",
        Some("render"),
        serde_json::json!({
            "name": "render",
            "arguments": {
                "shape": { "kind": "Rect", "w": 3.0, "h": 4.0 },
                "title": "box"
            },
            "_meta": meta()
        }),
    )
    .await;

    let result = &body["result"];
    assert!(
        result.get("error").is_none() && result["isError"] != true,
        "tools/call must succeed: {body}"
    );

    // Consistency: structuredContent must satisfy the ADVERTISED outputSchema —
    // same wrapper field, same required leaf fields. Discover the field from
    // tools/list rather than assuming a name.
    let listed = post(
        &url,
        "tools/list",
        None,
        serde_json::json!({"_meta": meta()}),
    )
    .await;
    let output = listed["result"]["tools"]
        .as_array()
        .and_then(|t| t.iter().find(|t| t["name"] == "render"))
        .map(|t| t["outputSchema"].clone())
        .expect("render outputSchema");
    let field = output["required"][0]
        .as_str()
        .expect("wrapper field required")
        .to_string();

    let structured = &result["structuredContent"][&field];
    assert!(
        structured.is_object(),
        "structuredContent must use the advertised wrapper field '{field}': {body}"
    );
    assert_eq!(structured["area"], 12.0, "{body}");
    assert_eq!(structured["label"], "box", "{body}");
}
