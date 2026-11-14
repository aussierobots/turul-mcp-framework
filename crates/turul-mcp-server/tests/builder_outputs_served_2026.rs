//! `ResourceBuilder` and `PromptBuilder` output registers on `McpServer` and is
//! served on the wire, the same as `ToolBuilder` output.
//!
//! Built only under the 2026 feature; compiles to nothing under 2025-11-25.
#![cfg(feature = "protocol-2026-07-28")]

mod common;

use serde_json::{Value, json};
use turul_mcp_builders::{PromptBuilder, ResourceBuilder};
use turul_mcp_protocol::resources::{ResourceContent, TextResourceContents};
use turul_mcp_server::prelude::*;

async fn start_server() -> String {
    let (listener, port) = common::bind_ephemeral();

    let static_resource = ResourceBuilder::new("file:///builder/readme.md")
        .name("readme")
        .mime_type("text/markdown")
        .text_content("# Builder")
        .build()
        .expect("build static resource");

    let dynamic_resource = ResourceBuilder::new("test://builder/dynamic")
        .name("dynamic")
        .mime_type("application/json")
        .read(|uri| async move {
            Ok(ResourceContent::Text(TextResourceContents {
                uri,
                mime_type: Some("application/json".into()),
                meta: Some([("k".to_string(), json!("v"))].into()),
                text: "{\"n\":1}".into(),
            }))
        })
        .build()
        .expect("build dynamic resource");

    let prompt = PromptBuilder::new("greet")
        .description("Greet someone")
        .string_argument("who", "Who to greet")
        .user_message("Hello, {who}!")
        .build()
        .expect("build prompt");

    let server = McpServer::builder()
        .name("builder-outputs-2026-test")
        .version("0.4.0")
        .resource(static_resource)
        .resource(dynamic_resource)
        .prompt(prompt)
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

async fn post(url: &str, method: &str, name: Option<&str>, mut params: Value) -> Value {
    params["_meta"] = json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientCapabilities": {}
    });
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
async fn resource_builder_static_content_is_listed_and_read() {
    let url = start_server().await;
    let list = post(&url, "resources/list", None, json!({})).await;
    let uris: Vec<&str> = list["result"]["resources"]
        .as_array()
        .unwrap_or_else(|| panic!("resources missing: {list}"))
        .iter()
        .filter_map(|r| r["uri"].as_str())
        .collect();
    assert!(uris.contains(&"file:///builder/readme.md"), "{list}");

    let uri = "file:///builder/readme.md";
    let read = post(&url, "resources/read", Some(uri), json!({ "uri": uri })).await;
    assert_eq!(read["result"]["contents"][0]["text"], "# Builder", "{read}");
    assert_eq!(
        read["result"]["contents"][0]["mimeType"], "text/markdown",
        "{read}"
    );
}

#[tokio::test]
async fn resource_builder_read_fn_content_item_meta_reaches_the_wire() {
    let url = start_server().await;
    let uri = "test://builder/dynamic";
    let read = post(&url, "resources/read", Some(uri), json!({ "uri": uri })).await;
    assert_eq!(
        read["result"]["contents"],
        json!([{
            "uri": uri, "mimeType": "application/json", "text": "{\"n\":1}", "_meta": { "k": "v" }
        }]),
        "{read}"
    );
}

#[tokio::test]
async fn prompt_builder_output_is_listed_and_rendered_with_arguments() {
    let url = start_server().await;
    let list = post(&url, "prompts/list", None, json!({})).await;
    assert_eq!(list["result"]["prompts"][0]["name"], "greet", "{list}");

    let get = post(
        &url,
        "prompts/get",
        Some("greet"),
        json!({ "name": "greet", "arguments": { "who": "Ada" } }),
    )
    .await;
    assert_eq!(
        get["result"]["messages"][0]["content"]["text"], "Hello, Ada!",
        "{get}"
    );
    assert_eq!(get["result"]["description"], "Greet someone", "{get}");
}
