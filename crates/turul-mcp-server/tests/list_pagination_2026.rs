//! Cursor pagination on the three list methods that are not `tools/list`.
//!
//! Utilities §Pagination: "Cursors are opaque tokens", clients "MUST treat
//! cursors as opaque", and "Invalid cursors SHOULD result in an error with
//! code -32602". A walk driven purely by `nextCursor` must therefore visit
//! every item exactly once, in the listing's own order, and stop — the
//! properties a client relies on when it cannot inspect the token it is given.
//!
//! `tools/list` is covered by `wire_edges_2026.rs`; this file covers
//! `resources/list`, `resources/templates/list` and `prompts/list`, which had
//! type-level coverage only.
//!
//! Built only under the 2026 feature; compiles to nothing under 2025-11-25.
#![cfg(feature = "protocol-2026-07-28")]

mod common;

use std::collections::HashMap;

use turul_mcp_protocol::prompts::{PromptArgument, PromptMessage};
use turul_mcp_protocol::resources::ResourceContent;
use turul_mcp_server::prelude::*;

/// A static resource at a fixed URI.
struct StaticResource(&'static str, &'static str);

impl HasResourceMetadata for StaticResource {
    fn name(&self) -> &str {
        self.0
    }
}
impl HasResourceUri for StaticResource {
    fn uri(&self) -> &str {
        self.1
    }
}
impl HasResourceDescription for StaticResource {}
impl HasResourceMimeType for StaticResource {}
impl HasResourceSize for StaticResource {}
impl HasResourceAnnotations for StaticResource {}
impl HasResourceMeta for StaticResource {}
impl HasIcons for StaticResource {}

#[async_trait::async_trait]
impl McpResource for StaticResource {
    async fn read(
        &self,
        _params: Option<serde_json::Value>,
        _session: Option<&SessionContext>,
    ) -> McpResult<Vec<ResourceContent>> {
        Ok(vec![ResourceContent::text(self.1, "body")])
    }
}

struct NamedPrompt(&'static str);

impl HasPromptMetadata for NamedPrompt {
    fn name(&self) -> &str {
        self.0
    }
}
impl HasPromptDescription for NamedPrompt {}
impl HasPromptArguments for NamedPrompt {}
impl HasPromptAnnotations for NamedPrompt {}
impl HasPromptMeta for NamedPrompt {}
impl HasIcons for NamedPrompt {}

#[async_trait::async_trait]
impl McpPrompt for NamedPrompt {
    async fn render(
        &self,
        _args: Option<HashMap<String, serde_json::Value>>,
    ) -> McpResult<Vec<PromptMessage>> {
        Ok(vec![PromptMessage::user_text("hi")])
    }
}

/// Registered so `prompts/list` has a member carrying declared arguments —
/// pagination must not drop the descriptor fields as it pages.
struct ArgPrompt;

impl HasPromptMetadata for ArgPrompt {
    fn name(&self) -> &str {
        "delta"
    }
}
impl HasPromptDescription for ArgPrompt {}
impl HasPromptArguments for ArgPrompt {
    fn arguments(&self) -> Option<&Vec<PromptArgument>> {
        static ARGS: std::sync::OnceLock<Vec<PromptArgument>> = std::sync::OnceLock::new();
        Some(ARGS.get_or_init(|| vec![PromptArgument::new("who")]))
    }
}
impl HasPromptAnnotations for ArgPrompt {}
impl HasPromptMeta for ArgPrompt {}
impl HasIcons for ArgPrompt {}

#[async_trait::async_trait]
impl McpPrompt for ArgPrompt {
    async fn render(
        &self,
        _args: Option<HashMap<String, serde_json::Value>>,
    ) -> McpResult<Vec<PromptMessage>> {
        Ok(vec![PromptMessage::user_text("hi")])
    }
}

async fn start_server() -> String {
    let reserved = common::reserve_port().await;
    let port = reserved.port;

    let server = McpServer::builder()
        .name("list-pagination-2026")
        .version("0.4.0")
        // Static resources → resources/list.
        .resource(StaticResource("alpha", "file:///list/alpha.txt"))
        .resource(StaticResource("beta", "file:///list/beta.txt"))
        .resource(StaticResource("gamma", "file:///list/gamma.txt"))
        // Templated URIs auto-register as templates → resources/templates/list,
        // and are deliberately absent from resources/list.
        .resource(StaticResource("tpl-one", "file:///list/one-{id}.json"))
        .resource(StaticResource("tpl-two", "file:///list/two-{id}.json"))
        .prompt(NamedPrompt("alpha"))
        .prompt(NamedPrompt("beta"))
        .prompt(NamedPrompt("gamma"))
        .prompt(ArgPrompt)
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

async fn post(url: &str, rpc_method: &str, params: serde_json::Value) -> serde_json::Value {
    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", rpc_method)
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": rpc_method, "params": params
        }))
        .send()
        .await
        .expect("POST");
    resp.json().await.expect("json body")
}

/// Identities in the order the unpaginated listing returns them.
async fn list_all(url: &str, method: &str, array: &str, id: &str) -> Vec<String> {
    let body = post(url, method, serde_json::json!({"_meta": meta()})).await;
    body["result"][array]
        .as_array()
        .unwrap_or_else(|| panic!("{method} must return {array}: {body}"))
        .iter()
        .map(|item| {
            item[id]
                .as_str()
                .unwrap_or_else(|| panic!("{method}: item lacks {id}: {item}"))
                .to_string()
        })
        .collect()
}

/// Identities collected by following `nextCursor` at `limit` items per page,
/// with the number of pages it took.
async fn walk(url: &str, method: &str, array: &str, id: &str, limit: u32) -> (Vec<String>, usize) {
    let mut walked = Vec::new();
    let mut cursor: Option<String> = None;
    // Bounded so a server that reissues the same cursor fails here instead of
    // hanging the suite.
    for page in 1..=32 {
        let mut params = serde_json::json!({ "limit": limit, "_meta": meta() });
        if let Some(c) = &cursor {
            params["cursor"] = serde_json::json!(c);
        }
        let body = post(url, method, params).await;
        for item in body["result"][array].as_array().expect("array") {
            walked.push(item[id].as_str().expect("identity").to_string());
        }
        match body["result"]["nextCursor"].as_str() {
            Some(next) => cursor = Some(next.to_string()),
            None => return (walked, page),
        }
    }
    panic!("{method}: pagination did not terminate after 32 pages");
}

async fn assert_walk_matches_listing(url: &str, method: &str, array: &str, id: &str) {
    let all = list_all(url, method, array, id).await;
    assert!(
        all.len() > 1,
        "{method} needs more than one item for the walk to mean anything: {all:?}"
    );

    let (walked, pages) = walk(url, method, array, id, 1).await;
    assert_eq!(
        walked, all,
        "{method}: walk must cover the listing in order"
    );
    assert_eq!(
        pages,
        all.len(),
        "{method}: limit=1 must actually page — a server ignoring limit would \
         return everything at once and still match the listing"
    );
}

async fn assert_invalid_cursor_rejected(url: &str, method: &str) {
    let body = post(
        url,
        method,
        serde_json::json!({ "cursor": "not-a-cursor-this-server-issued", "_meta": meta() }),
    )
    .await;
    assert_eq!(
        body["error"]["code"], -32602,
        "{method} must reject an invalid cursor: {body}"
    );
}

#[tokio::test]
async fn resources_list_paginates_and_rejects_an_invalid_cursor() {
    let url = start_server().await;
    assert_walk_matches_listing(&url, "resources/list", "resources", "uri").await;
    assert_invalid_cursor_rejected(&url, "resources/list").await;
}

#[tokio::test]
async fn resource_templates_list_paginates_and_rejects_an_invalid_cursor() {
    let url = start_server().await;
    assert_walk_matches_listing(
        &url,
        "resources/templates/list",
        "resourceTemplates",
        "uriTemplate",
    )
    .await;
    assert_invalid_cursor_rejected(&url, "resources/templates/list").await;
}

#[tokio::test]
async fn prompts_list_paginates_and_rejects_an_invalid_cursor() {
    let url = start_server().await;
    assert_walk_matches_listing(&url, "prompts/list", "prompts", "name").await;
    assert_invalid_cursor_rejected(&url, "prompts/list").await;
}

/// Server §Resources: "Do not enumerate dynamic template instances in
/// resources/list" — the two listings partition the registered resources, so a
/// walk of one never yields an entry belonging to the other.
#[tokio::test]
async fn the_two_resource_listings_do_not_overlap() {
    let url = start_server().await;
    let (resources, _) = walk(&url, "resources/list", "resources", "uri", 1).await;
    let (templates, _) = walk(
        &url,
        "resources/templates/list",
        "resourceTemplates",
        "uriTemplate",
        1,
    )
    .await;

    assert!(
        resources.iter().all(|u| !u.contains('{')),
        "resources/list must not enumerate templates: {resources:?}"
    );
    assert!(
        templates.iter().all(|t| t.contains('{')),
        "resources/templates/list must only carry templates: {templates:?}"
    );
    assert!(
        resources.iter().all(|u| !templates.contains(u)),
        "the two listings must not overlap: {resources:?} vs {templates:?}"
    );
}
