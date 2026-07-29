//! A macro-authored tool can carry icons all the way to `tools/list`.
//!
//! Both macros used to emit `impl HasIcons for #name {}` unconditionally with no
//! attribute to populate it, so a hand-written override collided
//! (`error[E0119]: conflicting implementations`) and there was no other route.
//! Icons were reachable only from a manual trait impl — which meant
//! `examples/icon-showcase` demonstrated a surface the framework's two primary
//! authoring paths could not reach.
//!
//! Asserting on the `tools/list` wire bytes rather than on the trait is the
//! point: attribute parsing and serialization are separate steps, and a test
//! that only constructs the type passes even when the icons never leave.
#![cfg(feature = "protocol-2026-07-28")]

mod common;

use turul_mcp_derive::{McpTool, mcp_tool};
use turul_mcp_server::prelude::*;

const DERIVED_ICON: &str = "https://example.com/derived.png";
const FN_ICON: &str = "https://example.com/function.png";

#[derive(McpTool, Clone, Default)]
#[tool(
    name = "derived_icon_tool",
    description = "Derive-macro tool carrying icons",
    output = String,
    icons = vec![turul_mcp_protocol::icons::Icon::new(DERIVED_ICON)]
)]
struct DerivedIconTool {}

impl DerivedIconTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok("derived".to_string())
    }
}

#[mcp_tool(
    name = "function_icon_tool",
    description = "Function-macro tool carrying icons",
    icons = vec![turul_mcp_protocol::icons::Icon::new(FN_ICON)]
)]
async fn function_icon_tool() -> McpResult<String> {
    Ok("function".to_string())
}

/// A tool declaring no icons must not gain an empty array on the wire — `icons`
/// is optional, and an empty list is a different claim from absence.
#[derive(McpTool, Clone, Default)]
#[tool(name = "plain_tool", description = "No icons declared", output = String)]
struct PlainTool {}

impl PlainTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok("plain".to_string())
    }
}

async fn tools_list() -> serde_json::Value {
    let reserved = common::reserve_port().await;
    let port = reserved.port;

    let server = McpServer::builder()
        .name("tool-icons-2026")
        .version("0.4.0")
        .tool(DerivedIconTool::default())
        .tool_fn(function_icon_tool)
        .tool(PlainTool::default())
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

    client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/list")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/list",
            "params": { "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientCapabilities": {}
            }}
        }))
        .send()
        .await
        .expect("tools/list POST")
        .json()
        .await
        .expect("json body")
}

fn tool<'a>(body: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    body["result"]["tools"]
        .as_array()
        .unwrap_or_else(|| panic!("no tools array: {body}"))
        .iter()
        .find(|t| t["name"] == name)
        .unwrap_or_else(|| panic!("{name} absent from tools/list: {body}"))
}

#[tokio::test]
async fn derive_macro_icons_reach_the_tools_list_wire() {
    let body = tools_list().await;
    let t = tool(&body, "derived_icon_tool");
    assert_eq!(
        t["icons"][0]["src"], DERIVED_ICON,
        "icons declared on #[derive(McpTool)] must serialize: {t}"
    );
}

#[tokio::test]
async fn function_macro_icons_reach_the_tools_list_wire() {
    let body = tools_list().await;
    let t = tool(&body, "function_icon_tool");
    assert_eq!(
        t["icons"][0]["src"], FN_ICON,
        "icons declared on #[mcp_tool] must serialize: {t}"
    );
}

#[tokio::test]
async fn a_tool_declaring_no_icons_omits_the_field() {
    let body = tools_list().await;
    let t = tool(&body, "plain_tool");
    assert!(
        t.get("icons").is_none() || t["icons"].is_null(),
        "absent icons must stay absent rather than becoming an empty array: {t}"
    );
}
