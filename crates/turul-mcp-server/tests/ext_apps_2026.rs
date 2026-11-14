//! MCP Apps (`io.modelcontextprotocol/ui`, apps-2026-01-26) served through the
//! server's generic hooks only: `extension_capability`, `ToolBuilder::meta`,
//! `HasToolMeta`, a resource's own `read`, and `SessionContext::client_capabilities`.
//! The server has no Apps feature; these tests prove none is needed to serve
//! the MCP-side surface the Apps spec assigns to a server.
//!
//! Built only under the 2026 feature; compiles to nothing under 2025-11-25.
#![cfg(feature = "protocol-2026-07-28")]

mod common;

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::{Value, json};
use turul_mcp_builders::ToolBuilder;
use turul_mcp_ext_apps::{
    EXTENSION_IDENTIFIER, MCP_APP_HTML_MIME, UiResourceCsp, UiResourceMeta, UiToolMeta,
    UiToolVisibility, capability, client_supports_html_views,
};
use turul_mcp_protocol::ToolSchema;
use turul_mcp_protocol::resources::{ResourceContent, TextResourceContents};
use turul_mcp_protocol::tools::{CallToolResult, ToolResult};
use turul_mcp_server::prelude::*;

const VIEW_URI: &str = "ui://weather/view.html";
const VIEW_HTML: &str = "<!DOCTYPE html><html><body>weather</body></html>";

fn view_meta() -> UiResourceMeta {
    UiResourceMeta {
        csp: Some(UiResourceCsp {
            connect_domains: Some(vec!["https://api.openweathermap.org".into()]),
            ..Default::default()
        }),
        prefers_border: Some(true),
        ..Default::default()
    }
}

/// A UI-linked tool that degrades in the call: the Apps spec requires
/// meaningful `content` either way, and the 2026-07-28 core has no handshake
/// at which to register a different variant.
struct WeatherTool {
    input_schema: ToolSchema,
    meta: HashMap<String, Value>,
}

impl WeatherTool {
    fn new() -> Self {
        Self {
            input_schema: ToolSchema::object(),
            meta: UiToolMeta {
                resource_uri: Some(VIEW_URI.into()),
                visibility: None,
            }
            .to_meta(),
        }
    }
}

impl HasBaseMetadata for WeatherTool {
    fn name(&self) -> &str {
        "get_weather"
    }
}
impl HasDescription for WeatherTool {
    fn description(&self) -> Option<&str> {
        Some("Current weather")
    }
}
impl HasInputSchema for WeatherTool {
    fn input_schema(&self) -> &ToolSchema {
        &self.input_schema
    }
}
impl HasOutputSchema for WeatherTool {}
impl HasAnnotations for WeatherTool {}
impl HasToolMeta for WeatherTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        Some(&self.meta)
    }
}
impl HasIcons for WeatherTool {}
impl HasExecution for WeatherTool {}

#[async_trait]
impl McpTool for WeatherTool {
    async fn call(&self, _args: Value, s: Option<SessionContext>) -> McpResult<CallToolResult> {
        let ui = s
            .and_then(|s| s.client_capabilities())
            .is_some_and(|c| client_supports_html_views(&c));
        let text = if ui {
            "Sunny, 22°C (view attached)"
        } else {
            "Sunny, 22°C"
        };
        Ok(CallToolResult::success(vec![ToolResult::text(text)]))
    }
}

/// The UI resource. `_meta.ui` belongs on the `resources/read` content item,
/// so it is set in `read`, not on the declaration.
struct WeatherView;

impl HasResourceMetadata for WeatherView {
    fn name(&self) -> &str {
        "weather_view"
    }
}
impl HasResourceUri for WeatherView {
    fn uri(&self) -> &str {
        VIEW_URI
    }
}
impl HasResourceDescription for WeatherView {}
impl HasResourceMimeType for WeatherView {
    fn mime_type(&self) -> Option<&str> {
        Some(MCP_APP_HTML_MIME)
    }
}
impl HasResourceSize for WeatherView {}
impl HasResourceAnnotations for WeatherView {}
impl HasResourceMeta for WeatherView {}
impl HasIcons for WeatherView {}

#[async_trait]
impl McpResource for WeatherView {
    async fn read(
        &self,
        _params: Option<Value>,
        _session: Option<&SessionContext>,
    ) -> McpResult<Vec<ResourceContent>> {
        Ok(vec![ResourceContent::Text(TextResourceContents {
            uri: VIEW_URI.into(),
            mime_type: Some(MCP_APP_HTML_MIME.into()),
            meta: Some(view_meta().to_meta()),
            text: VIEW_HTML.into(),
        })])
    }
}

async fn start_server() -> String {
    let (listener, port) = common::bind_ephemeral();

    let refresh = ToolBuilder::new("refresh_weather")
        .description("Refresh the weather view")
        .meta(
            UiToolMeta {
                resource_uri: Some(VIEW_URI.into()),
                visibility: Some(vec![UiToolVisibility::App]),
            }
            .to_meta(),
        )
        .execute(|_| async { Ok(json!("refreshed")) })
        .build()
        .expect("build refresh tool");

    let server = McpServer::builder()
        .name("ext-apps-2026-test")
        .version("0.4.0")
        .extension_capability(EXTENSION_IDENTIFIER, capability())
        .tool(WeatherTool::new())
        .tool(refresh)
        .resource(WeatherView)
        .listener(listener)
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

fn meta(client_caps: Value) -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientCapabilities": client_caps
    })
}

async fn post(url: &str, method: &str, name: Option<&str>, params: Value) -> Value {
    let mut req = reqwest::Client::new()
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", method);
    if let Some(n) = name {
        req = req.header("Mcp-Name", n);
    }
    req.json(&json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }))
        .send()
        .await
        .expect("POST")
        .json()
        .await
        .expect("json body")
}

#[tokio::test]
async fn adr028_server_declares_ui_extension_via_extension_capability() {
    let url = start_server().await;
    let body = post(
        &url,
        "server/discover",
        None,
        json!({ "_meta": meta(json!({})) }),
    )
    .await;
    assert_eq!(
        body["result"]["capabilities"]["extensions"][EXTENSION_IDENTIFIER],
        json!({}),
        "{body}"
    );
}

/// Visibility filtering is a host duty (apps-2026-01-26 §Visibility); the
/// server lists app-only tools because apps call them through `tools/call`.
#[tokio::test]
async fn apps_visibility_is_a_host_duty_server_lists_app_only_tools_with_ui_meta() {
    let url = start_server().await;
    let body = post(
        &url,
        "tools/list",
        None,
        json!({ "_meta": meta(json!({})) }),
    )
    .await;
    let tools = body["result"]["tools"].as_array().expect("tools array");
    let by_name = |n: &str| {
        tools
            .iter()
            .find(|t| t["name"] == n)
            .unwrap_or_else(|| panic!("{n} missing: {body}"))
    };
    assert_eq!(
        by_name("get_weather")["_meta"],
        json!({ "ui": { "resourceUri": VIEW_URI } })
    );
    assert_eq!(
        by_name("refresh_weather")["_meta"],
        json!({ "ui": { "resourceUri": VIEW_URI, "visibility": ["app"] } })
    );
}

#[tokio::test]
async fn apps_ui_resource_reads_with_mcp_app_mime_and_content_item_ui_meta() {
    let url = start_server().await;
    let body = post(
        &url,
        "resources/read",
        Some(VIEW_URI),
        json!({ "uri": VIEW_URI, "_meta": meta(json!({})) }),
    )
    .await;
    assert_eq!(
        body["result"]["contents"],
        json!([{
            "uri": VIEW_URI,
            "mimeType": MCP_APP_HTML_MIME,
            "text": VIEW_HTML,
            "_meta": { "ui": {
                "csp": { "connectDomains": ["https://api.openweathermap.org"] },
                "prefersBorder": true
            }}
        }]),
        "{body}"
    );
}

/// The tool reads the per-request client capability and returns text
/// `content` whether or not the host renders views.
#[tokio::test]
async fn apps_graceful_degradation_tool_returns_text_content_with_or_without_ui_capability() {
    let url = start_server().await;
    let call = |caps: Value| {
        let url = url.clone();
        async move {
            let body = post(
                &url,
                "tools/call",
                Some("get_weather"),
                json!({ "name": "get_weather", "arguments": {}, "_meta": meta(caps) }),
            )
            .await;
            body["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_else(|| panic!("text content missing: {body}"))
                .to_string()
        }
    };

    let with_ui = call(json!({
        "extensions": { EXTENSION_IDENTIFIER: { "mimeTypes": [MCP_APP_HTML_MIME] } }
    }))
    .await;
    assert_eq!(with_ui, "Sunny, 22°C (view attached)");

    let without_ui = call(json!({})).await;
    assert_eq!(without_ui, "Sunny, 22°C");
}
