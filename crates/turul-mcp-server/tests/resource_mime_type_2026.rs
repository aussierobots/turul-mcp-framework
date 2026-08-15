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
/// No file extension, on purpose — see [`BinaryResource`].
const BINARY_URI: &str = "test://sprite";
const BINARY_MIME: &str = "image/png";
const BINARY_BLOB: &str = "iVBORw0KGgo=";

/// A resource whose URI carries NO file extension and whose type is binary.
///
/// This is the shape that used to be unreadable. `build()` auto-generated a
/// resource policy whose `allowed_mime_types` was derived from file extensions
/// found in registered URIs, so `image/png` was only permitted if some URI
/// happened to end in `.png`. `resources/read` then validated the mimeType the
/// SERVER itself declared against that list and answered -32602 — the server
/// refusing its own output, with the outcome depending on unrelated URIs'
/// cosmetics. Non-file URI schemes (`test://`, `config://`, `ui://`) are normal
/// in MCP, so this was reachable by ordinary configuration.
struct BinaryResource;

impl HasResourceMetadata for BinaryResource {
    fn name(&self) -> &str {
        "sprite"
    }
}
impl HasResourceUri for BinaryResource {
    fn uri(&self) -> &str {
        BINARY_URI
    }
}
impl HasResourceDescription for BinaryResource {}
impl HasResourceMimeType for BinaryResource {
    fn mime_type(&self) -> Option<&str> {
        Some(BINARY_MIME)
    }
}
impl HasResourceSize for BinaryResource {}
impl HasResourceAnnotations for BinaryResource {}
impl HasResourceMeta for BinaryResource {}
impl HasIcons for BinaryResource {}

#[async_trait::async_trait]
impl McpResource for BinaryResource {
    async fn read(
        &self,
        _params: Option<serde_json::Value>,
        _session: Option<&SessionContext>,
    ) -> McpResult<Vec<ResourceContent>> {
        Ok(vec![ResourceContent::blob(
            BINARY_URI,
            BINARY_BLOB,
            BINARY_MIME.to_string(),
        )])
    }
}

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
        .resource(BinaryResource)
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

/// Same fixtures, but WITHOUT `.test_mode()`.
///
/// `test_mode()` builds the read handler with `.without_security()`
/// (builder.rs:1209), so a server started by [`start_server`] never runs the
/// auto-generated resource policy at all. The MIME defect this file guards
/// lives *in* that policy, so a test on the test_mode server cannot see it —
/// verified by re-introducing a restrictive allowlist and watching the test
/// still pass. This helper exercises the real path.
async fn start_server_with_security() -> String {
    let reserved = common::reserve_port().await;
    let port = reserved.port;

    let server = McpServer::builder()
        .name("resource-mime-2026-secured")
        .version("0.4.0")
        .resource(MarkdownResource)
        .resource(JsonResource)
        .resource(BinaryResource)
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
    // Three: markdown, json, and the extension-less binary added with the
    // 2026-08-15 MIME-allowlist fix. This loop checks list/read agreement for
    // every registered resource, so a new fixture belongs in the count.
    assert_eq!(resources.len(), 3, "{listed}");

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

/// A resource must be readable with the mimeType the server declared for it,
/// regardless of whether its URI looks like a filename.
///
/// Regression guard: this fails with
/// `-32602 Invalid parameter type for 'mime_type': expected allowed MIME type,
/// got image/png` against the extension-derived allowlist removed on
/// 2026-08-15. Found by upstream's conformance suite (`resources-read-binary`);
/// every resource fixture in this repo used `file:///…ext` URIs, so the one
/// configuration that broke was the one nothing exercised.
#[tokio::test]
async fn a_binary_resource_with_no_file_extension_is_readable() {
    let url = start_server_with_security().await;

    let body = post(
        &url,
        "resources/read",
        Some(BINARY_URI),
        serde_json::json!({ "uri": BINARY_URI, "_meta": meta() }),
    )
    .await;

    assert!(
        body.get("error").is_none(),
        "the server must not reject a mimeType it declared itself: {body}"
    );
    assert_eq!(
        body["result"]["contents"][0]["mimeType"], BINARY_MIME,
        "read must report the declared mimeType: {body}"
    );
    assert_eq!(
        body["result"]["contents"][0]["blob"], BINARY_BLOB,
        "the blob payload must survive the read: {body}"
    );
}
