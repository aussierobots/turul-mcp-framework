//! `tools/list` against a REAL 2026-07-28 server advertising a JSON Schema
//! 2020-12 composition (SEP-2106) the client's public `Tool` vocabulary cannot
//! represent.
//!
//! A data-bearing tagged union renders as a property-level `oneOf` with no
//! `"type"` key. The public `Tool` models properties as a closed enum keyed on
//! `"type"`, so that property has no representation — but one such tool MUST NOT
//! fail the whole listing, MUST stay in the list, and MUST stay callable. The
//! untruncated schema stays reachable via `McpClient::tool_input_schema`.
#![cfg(feature = "client-bilingual")]

use serde::{Deserialize, Serialize};
use serde_json::json;
use turul_mcp_builders::schemars;
use turul_mcp_client::config::ClientConfig;
use turul_mcp_client::transport::http::HttpTransport;
use turul_mcp_client::{McpClient, McpVersion};
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;

/// Data-bearing tagged union — renders as `oneOf` with `const` tags.
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

/// A plain tool alongside it: proves the composition tool does not take the
/// rest of the listing down with it.
#[derive(McpTool, Clone, Default)]
#[tool(name = "echo", description = "Echo back the message", output = String)]
struct EchoTool {
    #[param(description = "Message to echo back")]
    message: String,
}

impl EchoTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok(format!("Echo: {}", self.message))
    }
}

async fn start_2026_server() -> String {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();

    let server = McpServer::builder()
        .name("schema-2020-12-client")
        .version("0.4.0")
        .tool(RenderTool::default())
        .tool(EchoTool::default())
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

#[tokio::test]
async fn list_tools_survives_a_property_level_one_of_and_keeps_the_tool_callable() {
    let url = start_2026_server().await;

    let transport = Box::new(HttpTransport::new(&url).unwrap());
    let client = McpClient::new(transport, ClientConfig::default());
    client.connect().await.expect("negotiation must succeed");
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2026_07_28)
    );

    let tools = client
        .list_tools()
        .await
        .expect("a 2020-12 composition in one tool must not fail the whole listing");

    assert!(
        tools.iter().any(|t| t.name == "render"),
        "the composition-bearing tool must remain in the list: {:?}",
        tools.iter().map(|t| &t.name).collect::<Vec<_>>()
    );
    assert!(
        tools.iter().any(|t| t.name == "echo"),
        "the plain tool must remain in the list too"
    );

    // The dropped detail is recoverable: the raw advertised schema still
    // carries the composition.
    let raw = client
        .tool_input_schema("render")
        .await
        .expect("the raw 2026 inputSchema must be retained");
    let shape = raw["properties"]["shape"].to_string();
    assert!(
        shape.contains("oneOf") || shape.contains("anyOf"),
        "the untruncated schema must carry the composition: {raw}"
    );

    // Still callable — a listing entry the client cannot fully describe is
    // useless if it cannot be invoked.
    let result = client
        .call_tool(
            "render",
            json!({ "shape": { "kind": "Rect", "w": 3.0, "h": 4.0 }, "title": "box" }),
        )
        .await
        .expect("the downgraded tool must still be callable");
    let text = serde_json::to_string(&result).unwrap_or_default();
    assert!(text.contains("12"), "tool must execute: {text}");
}

/// The paginated variant shares the remap and must survive the same input.
#[tokio::test]
async fn list_tools_paginated_survives_a_property_level_one_of() {
    let url = start_2026_server().await;

    let transport = Box::new(HttpTransport::new(&url).unwrap());
    let client = McpClient::new(transport, ClientConfig::default());
    client.connect().await.expect("negotiation must succeed");

    let page = client
        .list_tools_paginated(None)
        .await
        .expect("a 2020-12 composition must not fail the paginated listing");
    assert!(
        page.tools.iter().any(|t| t.name == "render"),
        "the composition-bearing tool must remain in the paginated list"
    );
}
