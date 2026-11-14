//! # MCP Apps Server (`io.modelcontextprotocol/ui`, apps-2026-01-26)
//!
//! Serves an interactive HTML view with the server's generic hooks only; the
//! server has no Apps feature. Three pieces:
//! - `ui://weather/view.html`: the view, read with MIME
//!   `text/html;profile=mcp-app` and `_meta.ui` (CSP, border) on the content item
//! - `get_weather`: linked to the view via `_meta.ui.resourceUri`; returns
//!   text `content` either way, noting the view only when the request's
//!   `clientCapabilities` declare the Apps HTML profile
//! - `refresh_weather`: `visibility: ["app"]`. The server still lists it;
//!   hiding it from the model is the host's job, and the view calls it
//!
//! ```bash
//! cargo run -p ext-apps-server
//! ```

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
/// The App SDK the view loads, pinned to one published file. The import map's
/// `integrity` entry makes the browser refuse it if its bytes ever change, and
/// `SDK_ORIGIN` is the only origin the view's CSP admits.
const SDK_ORIGIN: &str = "https://cdn.jsdelivr.net";
const VIEW_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>Weather</title>
<script type="importmap">
{
  "imports": {
    "@modelcontextprotocol/ext-apps": "https://cdn.jsdelivr.net/npm/@modelcontextprotocol/ext-apps@2.1.0/dist/src/app-with-deps.js"
  },
  "integrity": {
    "https://cdn.jsdelivr.net/npm/@modelcontextprotocol/ext-apps@2.1.0/dist/src/app-with-deps.js": "sha384-bjsAPLYL21mbv3hZB/TVEqrI1rAAzwxupvAkIHxgOe9u8vInXZrRrMxxcjOc7TIj"
  }
}
</script>
</head>
<body>
<p id="out">Sunny, 22°C</p>
<button id="refresh">Refresh</button>
<script type="module">
import { App } from "@modelcontextprotocol/ext-apps";

const out = document.getElementById("out");
const show = (result) => {
  out.textContent = result?.content?.find((c) => c.type === "text")?.text ?? "(no text)";
};
const app = new App({ name: "Weather View", version: "1.0.0" });
app.ontoolresult = show;
app.onerror = console.error;
document.getElementById("refresh").addEventListener("click", async () => {
  try {
    show(await app.callServerTool({ name: "refresh_weather", arguments: {} }));
  } catch (e) {
    console.error(e);
    out.textContent = "Refresh failed";
  }
});
await app.connect();
</script>
</body>
</html>
"#;

/// A tool linked to the view. `_meta` comes from a manual `HasToolMeta`
/// impl because the call also reads the session's client capabilities.
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
        Some("Current weather, with an interactive view on hosts that render MCP Apps")
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

/// The view. `_meta.ui` rides the `resources/read` content item.
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
impl HasResourceDescription for WeatherView {
    fn description(&self) -> Option<&str> {
        Some("Interactive weather view")
    }
}
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
        let meta = UiResourceMeta {
            csp: Some(UiResourceCsp {
                resource_domains: Some(vec![SDK_ORIGIN.into()]),
                ..Default::default()
            }),
            prefers_border: Some(true),
            ..Default::default()
        };
        Ok(vec![ResourceContent::Text(TextResourceContents {
            uri: VIEW_URI.into(),
            mime_type: Some(MCP_APP_HTML_MIME.into()),
            meta: Some(meta.to_meta()),
            text: VIEW_HTML.into(),
        })])
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    // App-only: a builder tool is enough when the call needs no session.
    let refresh = ToolBuilder::new("refresh_weather")
        .description("Refresh the weather view (called by the view, not the model)")
        .meta(
            UiToolMeta {
                resource_uri: Some(VIEW_URI.into()),
                visibility: Some(vec![UiToolVisibility::App]),
            }
            .to_meta(),
        )
        .execute(|_| async {
            let secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs());
            Ok(json!(format!("Sunny, 22°C (refreshed at {secs})")))
        })
        .build()?;

    let server = McpServer::builder()
        .name("ext-apps-server")
        .version(env!("CARGO_PKG_VERSION"))
        .title("MCP Apps Example")
        .extension_capability(EXTENSION_IDENTIFIER, capability())
        .tool(WeatherTool::new())
        .tool(refresh)
        .resource(WeatherView)
        .bind_address("127.0.0.1:8660".parse()?)
        .build()?;

    tracing::info!("MCP Apps server at http://127.0.0.1:8660/mcp");
    server.run().await?;
    Ok(())
}
