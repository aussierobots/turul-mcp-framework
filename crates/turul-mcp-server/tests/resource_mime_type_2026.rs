//! `resources/list` and `resources/read` must agree on `mimeType` for the same
//! URI. Server §Resources describes `mimeType` as "the MIME type of this
//! resource" — one property of one resource, so a listing that advertises
//! `text/markdown` while the read reports `text/plain` is self-contradictory
//! and leaves a client no way to know which to believe.
//!
//! Built only under the 2026 feature; compiles to nothing under 2025-11-25.
#![cfg(feature = "protocol-2026-07-28")]

mod common;

use turul_mcp_protocol::resources::ResourceContent;
use turul_mcp_server::prelude::*;

const MARKDOWN_URI: &str = "file:///fixture/readme.md";
const MARKDOWN_MIME: &str = "text/markdown";
const JSON_URI: &str = "file:///fixture/config.json";
const JSON_MIME: &str = "application/json";

/// Text content whose type is NOT the `text/plain` that `ResourceContent::text`
/// defaults to — the case the declared/reported split used to break.
struct MarkdownResource;

impl HasResourceMetadata for MarkdownResource {
    fn name(&self) -> &str {
        "readme"
    }
}
impl HasResourceUri for MarkdownResource {
    fn uri(&self) -> &str {
        MARKDOWN_URI
    }
}
impl HasResourceDescription for MarkdownResource {}
impl HasResourceMimeType for MarkdownResource {
    fn mime_type(&self) -> Option<&str> {
        Some(MARKDOWN_MIME)
    }
}
impl HasResourceSize for MarkdownResource {}
impl HasResourceAnnotations for MarkdownResource {}
impl HasResourceMeta for MarkdownResource {}
impl HasIcons for MarkdownResource {}

#[async_trait::async_trait]
impl McpResource for MarkdownResource {
    async fn read(
        &self,
        _params: Option<serde_json::Value>,
        _session: Option<&SessionContext>,
    ) -> McpResult<Vec<ResourceContent>> {
        Ok(vec![
            ResourceContent::text(MARKDOWN_URI, "# Title\n").with_mime_type(MARKDOWN_MIME),
        ])
    }
}

/// The `json` constructor already picks the right type — included so the test
/// covers a resource that agrees without an explicit override.
struct JsonResource;

impl HasResourceMetadata for JsonResource {
    fn name(&self) -> &str {
        "config"
    }
}
impl HasResourceUri for JsonResource {
    fn uri(&self) -> &str {
        JSON_URI
    }
}
impl HasResourceDescription for JsonResource {}
impl HasResourceMimeType for JsonResource {
    fn mime_type(&self) -> Option<&str> {
        Some(JSON_MIME)
    }
}
impl HasResourceSize for JsonResource {}
impl HasResourceAnnotations for JsonResource {}
impl HasResourceMeta for JsonResource {}
impl HasIcons for JsonResource {}

#[async_trait::async_trait]
impl McpResource for JsonResource {
    async fn read(
        &self,
        _params: Option<serde_json::Value>,
        _session: Option<&SessionContext>,
    ) -> McpResult<Vec<ResourceContent>> {
        Ok(vec![ResourceContent::json(JSON_URI, "{}")])
    }
}

async fn start_server() -> String {
    let reserved = common::reserve_port().await;
    let port = reserved.port;

    let server = McpServer::builder()
        .name("resource-mime-2026")
        .version("0.4.0")
        .resource(MarkdownResource)
        .resource(JsonResource)
        .test_mode()
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
async fn read_reports_the_mime_type_that_list_advertises() {
    let url = start_server().await;

    let listed = post(
        &url,
        "resources/list",
        None,
        serde_json::json!({"_meta": meta()}),
    )
    .await;
    let resources = listed["result"]["resources"]
        .as_array()
        .expect("resources array")
        .clone();
    assert_eq!(resources.len(), 2, "{listed}");

    for resource in &resources {
        let uri = resource["uri"].as_str().expect("uri");
        let advertised = resource["mimeType"]
            .as_str()
            .unwrap_or_else(|| panic!("resources/list must advertise mimeType: {resource}"));

        let read = post(
            &url,
            "resources/read",
            Some(uri),
            serde_json::json!({ "uri": uri, "_meta": meta() }),
        )
        .await;
        let reported = read["result"]["contents"][0]["mimeType"]
            .as_str()
            .unwrap_or_else(|| panic!("resources/read must report mimeType: {read}"));

        assert_eq!(
            reported, advertised,
            "{uri}: resources/read reported {reported:?} but resources/list advertised \
             {advertised:?} — one resource cannot have two MIME types"
        );
    }

    // Guard the guard: if every fixture happened to be text/plain the loop above
    // would pass against a constructor that hardcodes it.
    let advertised: Vec<&str> = resources
        .iter()
        .filter_map(|r| r["mimeType"].as_str())
        .collect();
    assert!(
        advertised.contains(&MARKDOWN_MIME) && advertised.contains(&JSON_MIME),
        "fixtures must cover a non-text/plain type: {advertised:?}"
    );
}
