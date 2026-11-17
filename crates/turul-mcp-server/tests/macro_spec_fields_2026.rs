//! Every macro form can declare `_meta`, and the resource and prompt
//! forms can declare `icons`, `annotations`, `title` and (resources) `size`,
//! all the way to the served list JSON. `prompts/get` returns
//! `McpPrompt::get_response`, so a prompt's result `_meta` and its
//! `validate_args` reach the wire.
//!
//! Each test compares a whole served object against an exact expected value,
//! so a stray key or a dropped one fails just as a wrong value does. The
//! `plain_*` objects declare none of the new attributes and must serve
//! exactly what they served before.
#![cfg(feature = "protocol-2026-07-28")]

mod common;

use std::collections::HashMap;

use serde_json::{Value, json};
use turul_mcp_derive::{
    McpPrompt, McpResource, McpTool, mcp_resource, mcp_tool, prompt, resource, tool,
};
use turul_mcp_protocol::icons::Icon;
use turul_mcp_protocol::meta::Annotations;
use turul_mcp_protocol::prompts::{GetPromptResult, PromptMessage, Role};
use turul_mcp_protocol::resources::ResourceContent;
use turul_mcp_server::prelude::*;

const ICON: &str = "https://example.com/icon.png";

/// The transport stamps every result with the responding server's identity.
fn server_info() -> Value {
    json!({ "name": "macro-spec-fields-2026", "version": "0.4.0" })
}

fn ui_meta(uri: &str) -> HashMap<String, Value> {
    HashMap::from([("ui".to_string(), json!({ "resourceUri": uri }))])
}

#[mcp_tool(
    name = "fn_meta_tool",
    description = "Function-macro tool with _meta",
    meta = ui_meta("ui://test/fn.html")
)]
async fn fn_meta_tool() -> McpResult<String> {
    Ok("fn".to_string())
}

#[derive(McpTool, Clone, Default)]
#[tool(
    name = "derived_meta_tool",
    description = "Derive-macro tool with _meta",
    output = String,
    meta = ui_meta("ui://test/derived.html")
)]
struct DerivedMetaTool {}

impl DerivedMetaTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok("derived".to_string())
    }
}

#[mcp_resource(
    uri = "ui://test/view.html",
    name = "fn_view",
    description = "Function-macro resource with every new field",
    mime_type = "text/html;profile=mcp-app",
    meta = ui_meta("ui://test/view.html"),
    icons = vec![Icon::new(ICON)],
    annotations = Annotations::new().with_priority(0.5),
    size = 4
)]
async fn fn_view() -> McpResult<Vec<ResourceContent>> {
    Ok(vec![ResourceContent::text("ui://test/view.html", "<p/>")])
}

#[mcp_resource(
    uri = "test://items/{id}",
    name = "item_template",
    title = "Item",
    description = "Template resource with every new field",
    meta = ui_meta("ui://test/item.html"),
    icons = vec![Icon::new(ICON)],
    annotations = Annotations::new().with_audience(vec![Role::User])
)]
async fn item_template(id: String) -> McpResult<Vec<ResourceContent>> {
    Ok(vec![ResourceContent::text(
        format!("test://items/{id}"),
        id,
    )])
}

#[derive(McpResource, Clone, Default)]
#[resource(
    name = "derived_view",
    uri = "ui://test/derived-view.html",
    description = "Derive-macro resource with every new field",
    meta = ui_meta("ui://test/derived-view.html"),
    icons = vec![Icon::new(ICON)],
    annotations = Annotations::new().with_priority(1.0),
    size = 4
)]
struct DerivedView {}

#[async_trait::async_trait]
impl McpResource for DerivedView {
    async fn read(
        &self,
        _params: Option<Value>,
        _session: Option<&SessionContext>,
    ) -> McpResult<Vec<ResourceContent>> {
        Ok(vec![ResourceContent::text(self.uri(), "<p/>")])
    }
}

#[derive(McpResource, Clone, Default)]
#[resource(
    name = "plain_resource",
    uri = "test://plain",
    description = "No new fields"
)]
struct PlainResource {}

#[async_trait::async_trait]
impl McpResource for PlainResource {
    async fn read(
        &self,
        _params: Option<Value>,
        _session: Option<&SessionContext>,
    ) -> McpResult<Vec<ResourceContent>> {
        Ok(vec![ResourceContent::text(self.uri(), "plain")])
    }
}

#[derive(McpPrompt, Clone, Default)]
#[prompt(
    name = "derived_prompt",
    title = "Derived Prompt",
    description = "Derive-macro prompt with every new field",
    meta = ui_meta("ui://test/prompt.html"),
    icons = vec![Icon::new(ICON)]
)]
struct DerivedPrompt {}

#[async_trait::async_trait]
impl McpPrompt for DerivedPrompt {
    async fn render(&self, _args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        Ok(vec![PromptMessage::text("derived")])
    }
}

#[derive(McpPrompt, Clone, Default)]
#[prompt(name = "plain_prompt", description = "No new fields")]
struct PlainPrompt {}

#[async_trait::async_trait]
impl McpPrompt for PlainPrompt {
    async fn render(&self, _args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        Ok(vec![PromptMessage::text("plain")])
    }
}

/// Overrides `get_response` to set result `_meta`, and `validate_args` to
/// reject `mode = "bad"`; `prompts/get` must call both on the wire.
#[derive(McpPrompt, Clone, Default)]
#[prompt(name = "result_meta_prompt", description = "Sets result _meta")]
struct ResultMetaPrompt {}

#[async_trait::async_trait]
impl McpPrompt for ResultMetaPrompt {
    async fn render(&self, _args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        Ok(vec![PromptMessage::text("rendered")])
    }

    async fn validate_args(&self, args: &HashMap<String, Value>) -> McpResult<()> {
        if args.get("mode").and_then(Value::as_str) == Some("bad") {
            return Err(McpError::InvalidParameters("mode must not be bad".into()));
        }
        Ok(())
    }

    async fn get_response(
        &self,
        args: Option<HashMap<String, Value>>,
    ) -> McpResult<GetPromptResult> {
        if let Some(ref a) = args {
            self.validate_args(a).await?;
        }
        let messages = self.render(args).await?;
        Ok(GetPromptResult::new(messages).with_meta(ui_meta("ui://test/result.html")))
    }
}

async fn start() -> String {
    let (listener, port) = common::bind_ephemeral();
    let server = McpServer::builder()
        .name("macro-spec-fields-2026")
        .version("0.4.0")
        .tool_fn(fn_meta_tool)
        .tool(DerivedMetaTool::default())
        .tool(tool! {
            name: "decl_meta_tool",
            description: "Declarative tool with _meta and icons",
            meta: ui_meta("ui://test/decl.html"),
            icons: vec![Icon::new(ICON)],
            params: {
                x: String => "Echoed",
            },
            execute: |x: String| async move { Ok::<_, String>(x) }
        })
        .resource_fn(fn_view)
        .resource_fn(item_template)
        .resource(DerivedView::default())
        .resource(PlainResource::default())
        .prompt(DerivedPrompt::default())
        .prompt(PlainPrompt::default())
        .prompt(ResultMetaPrompt::default())
        .resource(resource! {
            uri: "ui://test/decl-view.html",
            name: "decl_view",
            title: "Declarative View",
            description: "Declarative resource with every new field",
            mime_type: "text/html;profile=mcp-app",
            size: 4,
            meta: ui_meta("ui://test/decl-view.html"),
            icons: vec![Icon::new(ICON)],
            annotations: Annotations::new().with_priority(0.25),
            content: |_, _| async {
                Ok(vec![ResourceContent::text("ui://test/decl-view.html", "<p/>")])
            }
        })
        .resource(resource! {
            uri: "test://decl-plain",
            name: "decl_plain",
            description: "Declarative resource with no new fields",
            content: |_, _| async { Ok(vec![ResourceContent::text("test://decl-plain", "plain")]) }
        })
        .prompt(prompt! {
            name: "decl_prompt",
            title: "Declarative Prompt",
            description: "Declarative prompt with every new field",
            meta: ui_meta("ui://test/decl-prompt.html"),
            icons: vec![Icon::new(ICON)],
            template: |_args: Option<HashMap<String, Value>>| async move {
                Ok::<Vec<PromptMessage>, McpError>(vec![PromptMessage::text("decl")])
            }
        })
        .prompt(prompt! {
            name: "decl_plain_prompt",
            description: "Declarative prompt with no new fields",
            template: |_args: Option<HashMap<String, Value>>| async move {
                Ok::<Vec<PromptMessage>, McpError>(vec![PromptMessage::text("plain")])
            }
        })
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

async fn rpc(url: &str, method: &str, params: Value) -> Value {
    let mut params = params;
    let meta = params["_meta"].as_object().cloned().unwrap_or_default();
    let mut meta = Value::Object(meta);
    meta["io.modelcontextprotocol/protocolVersion"] = json!("2026-07-28");
    meta["io.modelcontextprotocol/clientCapabilities"] = json!({});
    params["_meta"] = meta;
    let mut req = reqwest::Client::new()
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", method);
    if let Some(name) = params["name"].as_str() {
        req = req.header("Mcp-Name", name);
    }
    req.json(&json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }))
        .send()
        .await
        .expect("POST")
        .json()
        .await
        .expect("json body")
}

async fn list(url: &str, method: &str, key: &str) -> Vec<Value> {
    let body = rpc(url, method, json!({})).await;
    body["result"][key]
        .as_array()
        .unwrap_or_else(|| panic!("no {key} array: {body}"))
        .clone()
}

fn find<'a>(items: &'a [Value], field: &str, value: &str) -> &'a Value {
    items
        .iter()
        .find(|i| i[field] == value)
        .unwrap_or_else(|| panic!("{value} absent: {items:?}"))
}

#[tokio::test]
async fn tool_macros_serve_declared_meta() {
    let url = start().await;
    let tools = list(&url, "tools/list", "tools").await;
    for (name, uri) in [
        ("fn_meta_tool", "ui://test/fn.html"),
        ("derived_meta_tool", "ui://test/derived.html"),
    ] {
        let t = find(&tools, "name", name);
        assert_eq!(t["_meta"], json!({ "ui": { "resourceUri": uri } }), "{t}");
    }
    let t = find(&tools, "name", "decl_meta_tool");
    assert_eq!(
        t["_meta"],
        json!({ "ui": { "resourceUri": "ui://test/decl.html" } }),
        "{t}"
    );
    assert_eq!(t["icons"], json!([{ "src": ICON }]), "{t}");
}

#[tokio::test]
async fn resource_macros_serve_declared_fields() {
    let url = start().await;
    let resources = list(&url, "resources/list", "resources").await;
    assert_eq!(
        *find(&resources, "name", "fn_view"),
        json!({
            "uri": "ui://test/view.html",
            "name": "fn_view",
            "description": "Function-macro resource with every new field",
            "mimeType": "text/html;profile=mcp-app",
            "annotations": { "priority": 0.5 },
            "size": 4,
            "icons": [{ "src": ICON }],
            "_meta": { "ui": { "resourceUri": "ui://test/view.html" } }
        })
    );
    assert_eq!(
        *find(&resources, "name", "derived_view"),
        json!({
            "uri": "ui://test/derived-view.html",
            "name": "derived_view",
            "description": "Derive-macro resource with every new field",
            "annotations": { "priority": 1.0 },
            "size": 4,
            "icons": [{ "src": ICON }],
            "_meta": { "ui": { "resourceUri": "ui://test/derived-view.html" } }
        })
    );
    assert_eq!(
        *find(&resources, "name", "plain_resource"),
        json!({
            "uri": "test://plain",
            "name": "plain_resource",
            "description": "No new fields"
        })
    );
    assert_eq!(
        *find(&resources, "name", "decl_view"),
        json!({
            "uri": "ui://test/decl-view.html",
            "name": "decl_view",
            "title": "Declarative View",
            "description": "Declarative resource with every new field",
            "mimeType": "text/html;profile=mcp-app",
            "annotations": { "priority": 0.25 },
            "size": 4,
            "icons": [{ "src": ICON }],
            "_meta": { "ui": { "resourceUri": "ui://test/decl-view.html" } }
        })
    );
    assert_eq!(
        *find(&resources, "name", "decl_plain"),
        json!({
            "uri": "test://decl-plain",
            "name": "decl_plain",
            "description": "Declarative resource with no new fields"
        })
    );
}

#[tokio::test]
async fn resource_templates_serve_declared_fields() {
    let url = start().await;
    let templates = list(&url, "resources/templates/list", "resourceTemplates").await;
    assert_eq!(
        *find(&templates, "name", "item_template"),
        json!({
            "uriTemplate": "test://items/{id}",
            "name": "item_template",
            "title": "Item",
            "description": "Template resource with every new field",
            "annotations": { "audience": ["user"] },
            "icons": [{ "src": ICON }],
            "_meta": { "ui": { "resourceUri": "ui://test/item.html" } }
        })
    );
}

#[tokio::test]
async fn prompt_derive_serves_declared_fields() {
    let url = start().await;
    let prompts = list(&url, "prompts/list", "prompts").await;
    assert_eq!(
        *find(&prompts, "name", "derived_prompt"),
        json!({
            "name": "derived_prompt",
            "title": "Derived Prompt",
            "description": "Derive-macro prompt with every new field",
            "icons": [{ "src": ICON }],
            "_meta": { "ui": { "resourceUri": "ui://test/prompt.html" } }
        })
    );
    assert_eq!(
        *find(&prompts, "name", "plain_prompt"),
        json!({ "name": "plain_prompt", "description": "No new fields" })
    );
    assert_eq!(
        *find(&prompts, "name", "decl_prompt"),
        json!({
            "name": "decl_prompt",
            "title": "Declarative Prompt",
            "description": "Declarative prompt with every new field",
            "icons": [{ "src": ICON }],
            "_meta": { "ui": { "resourceUri": "ui://test/decl-prompt.html" } }
        })
    );
    assert_eq!(
        *find(&prompts, "name", "decl_plain_prompt"),
        json!({
            "name": "decl_plain_prompt",
            "description": "Declarative prompt with no new fields"
        })
    );
}

#[tokio::test]
async fn prompt_get_serves_result_meta_and_runs_validate_args() {
    let url = start().await;
    // The prompt's own `ui` key wins over the caller's echoed one; the
    // caller's other extras are still echoed.
    let body = rpc(
        &url,
        "prompts/get",
        json!({
            "name": "result_meta_prompt",
            "arguments": { "mode": "good" },
            "_meta": { "trace": "t1", "ui": "caller" }
        }),
    )
    .await;
    assert_eq!(
        body["result"]["_meta"],
        json!({
            "io.modelcontextprotocol/serverInfo": server_info(),
            "ui": { "resourceUri": "ui://test/result.html" },
            "trace": "t1"
        }),
        "{body}"
    );
    assert_eq!(
        body["result"]["messages"][0]["content"]["text"], "rendered",
        "{body}"
    );

    let body = rpc(
        &url,
        "prompts/get",
        json!({ "name": "result_meta_prompt", "arguments": { "mode": "bad" } }),
    )
    .await;
    assert_eq!(body["error"]["code"], -32602, "{body}");
    assert!(body.get("result").is_none(), "{body}");
}

#[tokio::test]
async fn prompt_get_without_result_meta_is_unchanged() {
    let url = start().await;
    let body = rpc(&url, "prompts/get", json!({ "name": "plain_prompt" })).await;
    assert_eq!(
        body["result"],
        json!({
            "resultType": "complete",
            "_meta": { "io.modelcontextprotocol/serverInfo": server_info() },
            "description": "No new fields",
            "messages": [{ "role": "user", "content": { "type": "text", "text": "plain" } }]
        }),
        "{body}"
    );
}
