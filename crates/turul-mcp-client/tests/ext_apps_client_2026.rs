//! MCP Apps (`io.modelcontextprotocol/ui`, apps-2026-01-26) from the client side,
//! against a real in-process 2026-07-28 server: the opt-in declaration reaches
//! the server, and the typed `_meta.ui` accessors and visibility split read
//! what it serves.
//!
//! Run with: `cargo test -p turul-mcp-client --features ext-apps --test ext_apps_client_2026`
#![cfg(all(feature = "client-bilingual", feature = "ext-apps"))]

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::{Value, json};
use turul_mcp_client::McpClient;
use turul_mcp_client::config::ClientConfig;
use turul_mcp_client::ext_apps::{partition_tools, resource_ui_meta, tool_ui_meta};
use turul_mcp_client::transport::http::HttpTransport;
use turul_mcp_ext_apps::{
    EXTENSION_IDENTIFIER, MCP_APP_HTML_MIME, UiResourceMeta, UiToolMeta, UiToolVisibility,
    capability, client_supports_html_views,
};
use turul_mcp_protocol::ToolSchema;
use turul_mcp_protocol::resources::{ResourceContent, TextResourceContents};
use turul_mcp_protocol::tools::{CallToolResult, ToolResult};
use turul_mcp_server::prelude::*;

const VIEW_URI: &str = "ui://weather/view.html";

/// A tool whose text says whether the request declared Apps support.
struct UiTool {
    name: &'static str,
    schema: ToolSchema,
    meta: HashMap<String, Value>,
}

impl UiTool {
    fn new(name: &'static str, meta: HashMap<String, Value>) -> Self {
        Self {
            name,
            schema: ToolSchema::object(),
            meta,
        }
    }
}

impl HasBaseMetadata for UiTool {
    fn name(&self) -> &str {
        self.name
    }
}
impl HasDescription for UiTool {}
impl HasInputSchema for UiTool {
    fn input_schema(&self) -> &ToolSchema {
        &self.schema
    }
}
impl HasOutputSchema for UiTool {}
impl HasAnnotations for UiTool {}
impl HasToolMeta for UiTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        (!self.meta.is_empty()).then_some(&self.meta)
    }
}
impl HasIcons for UiTool {}
impl HasExecution for UiTool {}

#[async_trait]
impl McpTool for UiTool {
    async fn call(&self, _args: Value, s: Option<SessionContext>) -> McpResult<CallToolResult> {
        let ui = s
            .and_then(|s| s.client_capabilities())
            .is_some_and(|c| client_supports_html_views(&c));
        Ok(CallToolResult::success(vec![ToolResult::text(if ui {
            "ui"
        } else {
            "text"
        })]))
    }
}

struct View;

impl HasResourceMetadata for View {
    fn name(&self) -> &str {
        "view"
    }
}
impl HasResourceUri for View {
    fn uri(&self) -> &str {
        VIEW_URI
    }
}
impl HasResourceDescription for View {}
impl HasResourceMimeType for View {
    fn mime_type(&self) -> Option<&str> {
        Some(MCP_APP_HTML_MIME)
    }
}
impl HasResourceSize for View {}
impl HasResourceAnnotations for View {}
impl HasResourceMeta for View {}
impl HasIcons for View {}

#[async_trait]
impl McpResource for View {
    async fn read(
        &self,
        _params: Option<Value>,
        _session: Option<&SessionContext>,
    ) -> McpResult<Vec<ResourceContent>> {
        let meta = UiResourceMeta {
            prefers_border: Some(true),
            ..Default::default()
        };
        Ok(vec![ResourceContent::Text(TextResourceContents {
            uri: VIEW_URI.into(),
            mime_type: Some(MCP_APP_HTML_MIME.into()),
            meta: Some(meta.to_meta()),
            text: "<!DOCTYPE html><html></html>".into(),
        })])
    }
}

fn ui(visibility: Option<Vec<UiToolVisibility>>) -> HashMap<String, Value> {
    UiToolMeta {
        resource_uri: Some(VIEW_URI.into()),
        visibility,
    }
    .to_meta()
}

async fn start_server() -> String {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let server = McpServer::builder()
        .name("ext-apps-client-2026")
        .version("0.4.0")
        .extension_capability(EXTENSION_IDENTIFIER, capability())
        .tool(UiTool::new("get_weather", ui(None)))
        .tool(UiTool::new(
            "refresh_weather",
            ui(Some(vec![UiToolVisibility::App])),
        ))
        .tool(UiTool::new("plain", HashMap::new()))
        .resource(View)
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

async fn client(url: &str, declare: bool) -> McpClient {
    let mut config = ClientConfig::default();
    config.declared_capabilities.ext_apps = declare;
    let client = McpClient::new(Box::new(HttpTransport::new(url).unwrap()), config);
    client.connect().await.expect("connect");
    client
}

async fn call_text(client: &McpClient) -> String {
    let r = client
        .call_tool("get_weather", json!({}))
        .await
        .expect("call");
    serde_json::to_value(&r).unwrap()["content"][0]["text"]
        .as_str()
        .expect("text content")
        .to_string()
}

/// Apps §Client (Host) Capabilities: a host declares
/// `io.modelcontextprotocol/ui` with `mimeTypes`. The opt-in reaches the
/// server's per-request capabilities; without it nothing is declared.
#[tokio::test]
async fn apps_client_declares_ui_extension_with_mime_types_only_when_opted_in() {
    let url = start_server().await;
    assert_eq!(call_text(&client(&url, true).await).await, "ui");
    assert_eq!(call_text(&client(&url, false).await).await, "text");
}

/// Apps §Visibility: absent `visibility` is `["model", "app"]`; a tool
/// without `"model"` is app-only. The split keeps tools without `_meta.ui`
/// model-visible.
#[tokio::test]
async fn apps_visibility_split_separates_app_only_tools_from_model_visible_ones() {
    let url = start_server().await;
    let tools = client(&url, true)
        .await
        .list_tools()
        .await
        .expect("tools/list");

    let ui = tools.iter().find(|t| t.name == "get_weather").unwrap();
    assert_eq!(
        tool_ui_meta(ui).unwrap().unwrap().resource_uri.as_deref(),
        Some(VIEW_URI)
    );
    let plain = tools.iter().find(|t| t.name == "plain").unwrap();
    assert_eq!(tool_ui_meta(plain).unwrap(), None);

    let split = partition_tools(tools).expect("well-formed _meta.ui");
    let names = |v: &[turul_mcp_protocol_2025_11_25::tools::Tool]| {
        let mut n: Vec<String> = v.iter().map(|t| t.name.clone()).collect();
        n.sort();
        n
    };
    assert_eq!(names(&split.model_visible), ["get_weather", "plain"]);
    assert_eq!(names(&split.app_only), ["refresh_weather"]);
}

/// Apps §UI Resource Format: `_meta.ui` rides the `resources/read` content item.
#[tokio::test]
async fn apps_resource_ui_meta_is_read_from_the_read_content_item() {
    let url = start_server().await;
    let contents = client(&url, true)
        .await
        .read_resource(VIEW_URI)
        .await
        .expect("resources/read");
    let meta = resource_ui_meta(&contents[0]).unwrap().expect("_meta.ui");
    assert_eq!(meta.prefers_border, Some(true));
}
