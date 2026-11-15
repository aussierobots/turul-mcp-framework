//! ADR-007: `resources/read` refuses any `..` path segment, in every position
//! and in its percent-encoded form, before a URI reaches a resource handler,
//! and a template variable admits percent-encoded octets.
//!
//! The static `test://items/index` matters: its auto-generated allow pattern
//! (`^test://items/[^/]+$`) admits `%`, which is how `%2e%2e` reached the
//! `{id}` template handler as `..`. `test://entities/{entity_id}` has no
//! static sibling, so only its own template pattern admits a URI.
#![cfg(feature = "protocol-2026-07-28")]

mod common;

use std::sync::Mutex;

use serde_json::{Value, json};
use turul_mcp_derive::mcp_resource;
use turul_mcp_protocol::resources::ResourceContent;
use turul_mcp_server::prelude::*;

/// Every value a handler received, as `"<handler>:<value>"`.
static RECEIVED: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn received(handler: &str, value: &str) -> McpResult<Vec<ResourceContent>> {
    RECEIVED.lock().unwrap().push(format!("{handler}:{value}"));
    Ok(vec![ResourceContent::text("test://x", value)])
}

#[mcp_resource(uri = "test://items/{id}", name = "item", description = "One item")]
async fn item(id: String) -> McpResult<Vec<ResourceContent>> {
    received("item", &id)
}

#[mcp_resource(uri = "test://items/index", name = "index", description = "Item index")]
async fn index() -> McpResult<Vec<ResourceContent>> {
    Ok(vec![ResourceContent::text("test://items/index", "index")])
}

#[mcp_resource(
    uri = "test://entities/{entity_id}",
    name = "entity",
    description = "One entity"
)]
async fn entity(entity_id: String) -> McpResult<Vec<ResourceContent>> {
    received("entity", &entity_id)
}

#[mcp_resource(
    uri = "test://processed/{key}",
    name = "processed",
    description = "A key"
)]
async fn processed(key: String) -> McpResult<Vec<ResourceContent>> {
    received("processed", &key)
}

#[mcp_resource(uri = "test://files/{+path}", name = "file", description = "A file")]
async fn file(path: String) -> McpResult<Vec<ResourceContent>> {
    received("file", &path)
}

async fn start(test_mode: bool) -> String {
    let (listener, port) = common::bind_ephemeral();
    let builder = McpServer::builder();
    let builder = if test_mode {
        builder.test_mode()
    } else {
        builder
    };
    let server = builder
        .name("dot-segments")
        .version("0.4.0")
        .resource_fn(item)
        .resource_fn(index)
        .resource_fn(entity)
        .resource_fn(file)
        .resource_fn(processed)
        .listener(listener)
        .build()
        .expect("build");
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

async fn read(url: &str, uri: &str) -> Value {
    reqwest::Client::new()
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "resources/read")
        .header("Mcp-Name", uri)
        .json(&json!({
            "jsonrpc": "2.0", "id": 1, "method": "resources/read",
            "params": { "uri": uri, "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientCapabilities": {}
            }}
        }))
        .send()
        .await
        .expect("POST")
        .json()
        .await
        .expect("json")
}

/// No handler ever received a rooted value, a NUL, or a value that is, or
/// contains, a `.` or `..` segment.
fn assert_nothing_unsafe_received() {
    for entry in RECEIVED.lock().unwrap().iter() {
        let (_, value) = entry.split_once(':').unwrap();
        assert!(
            !value.split(['/', '\\']).any(|s| s == ".." || s == "."),
            "{entry}: a handler received a dot segment"
        );
        assert!(
            !value.starts_with(['/', '\\']) && !value.contains('\0'),
            "{entry}: a handler received a rooted or NUL value"
        );
    }
}

async fn assert_refused(url: &str) {
    for uri in [
        "test://items/..",
        "test://../items/x",
        "..",
        "test://items/a/../b",
        "test://items/%2e%2e",
        "test://items/%2E%2E",
        "test://items/.%2e",
        "test://entities/..",
        "test://entities/%2e%2e",
        "test://entities/.",
        "test://entities/%2e",
        "test://entities/a%2F..%2Fb",
        "test://entities/a%2F.%2Fb",
        "test://processed/%2e%2e",
        "test://processed/..",
        "test://processed/a%2F..%2Fb",
        "test://processed/a/../b",
        "test://files/..",
        "test://files/a/..",
        "test://files/%2e%2e",
        "test://files/%2e%2e/secret",
        "test://files/a/%2E%2E/b",
        "test://files/%2e%2e%2Fsecret",
        "test://files/a%2F..%2Fb",
        "test://files/a/./b",
        "test://files/./x",
        "test://entities/%2Fetc%2Fpasswd",
        "test://processed/%2Fetc",
        "test://files//etc/passwd",
        "test://files/%2Fetc",
        "test://entities/a%00b",
    ] {
        let body = read(url, uri).await;
        assert_eq!(body["error"]["code"], -32602, "{uri}: {body}");
    }
    assert_nothing_unsafe_received();
}

/// Sequential, so each server is probed on its own. `.test_mode()` removes
/// the configurable policy, not this refusal.
#[tokio::test]
async fn dot_segments_are_refused_before_the_handler() {
    assert_refused(&start(false).await).await;
    assert_refused(&start(true).await).await;
}

#[tokio::test]
async fn percent_encoded_octets_reach_a_simple_variable_decoded() {
    let url = start(false).await;
    for (uri, want) in [
        ("test://entities/acct%23019c8e00", "entity:acct#019c8e00"),
        ("test://entities/node%23abc", "entity:node#abc"),
        ("test://entities/plain-id_1.2", "entity:plain-id_1.2"),
        ("test://items/item%23x", "item:item#x"),
        // RFC 6570: a simple variable carries '/' percent-encoded.
        ("test://entities/a%2Fb", "entity:a/b"),
        (
            "test://processed/weather%2Falerts%2F2026-11-16.json",
            "processed:weather/alerts/2026-11-16.json",
        ),
    ] {
        let body = read(&url, uri).await;
        assert!(body.get("result").is_some(), "{uri}: {body}");
        assert!(
            RECEIVED.lock().unwrap().iter().any(|e| e == want),
            "{uri}: {want} not received"
        );
    }
}

#[tokio::test]
async fn a_reserved_variable_keeps_slashes() {
    let url = start(false).await;
    for (uri, want) in [
        ("test://files/a/b", "file:a/b"),
        ("test://files/dir/doc%23x.txt", "file:dir/doc#x.txt"),
    ] {
        let body = read(&url, uri).await;
        assert!(body.get("result").is_some(), "{uri}: {body}");
        assert!(
            RECEIVED.lock().unwrap().iter().any(|e| e == want),
            "{uri}: {want} not received"
        );
    }
}
