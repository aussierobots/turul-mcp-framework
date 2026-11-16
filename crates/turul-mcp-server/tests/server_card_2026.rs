//! Server Cards (experimental, ADR-038): the hosted card's HTTP contract and its
//! consistency with `server/discover`.
#![cfg(feature = "ext-server-card")]

mod common;

use serde_json::{Value, json};
use turul_mcp_ext_server_card::v1::{Remote, RemoteType, ServerCard};
use turul_mcp_protocol::icons::Icon;
use turul_mcp_server::prelude::*;

const CARD_PATH: &str = "/mcp/server-card";

fn card() -> ServerCard {
    ServerCard::new("com.example/weather", "Weather forecasts by city").with_remote(Remote::new(
        RemoteType::StreamableHttp,
        "https://mcp.example.com/mcp",
    ))
}

fn builder() -> McpServerBuilder {
    McpServer::builder()
        .name("weather")
        .version("1.2.0")
        .title("Weather")
        .icons(vec![Icon::new("https://example.com/icon.png")])
}

async fn start(builder: McpServerBuilder) -> String {
    let (listener, port) = common::bind_ephemeral();
    let server = builder.listener(listener).build().expect("build server");
    tokio::spawn(async move {
        server.run().await.ok();
    });
    let base = format!("http://127.0.0.1:{port}");
    let client = reqwest::Client::new();
    for _ in 0..50 {
        if client.get(format!("{base}/mcp")).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    base
}

fn header(resp: &reqwest::Response, name: &str) -> String {
    resp.headers()
        .get(name)
        .unwrap_or_else(|| panic!("missing {name}: {:?}", resp.headers()))
        .to_str()
        .unwrap()
        .to_string()
}

fn assert_card_cors(resp: &reqwest::Response) {
    assert_eq!(header(resp, "access-control-allow-origin"), "*");
    assert_eq!(header(resp, "access-control-allow-methods"), "GET");
    assert_eq!(
        header(resp, "access-control-allow-headers"),
        "Content-Type, If-None-Match"
    );
    assert_eq!(header(resp, "access-control-expose-headers"), "ETag");
}

async fn discover(base: &str) -> Value {
    reqwest::Client::new()
        .post(format!("{base}/mcp"))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "server/discover")
        .json(&json!({
            "jsonrpc": "2.0", "id": 1, "method": "server/discover",
            "params": { "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientInfo": { "name": "t", "version": "1" },
                "io.modelcontextprotocol/clientCapabilities": {}
            } }
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

#[tokio::test]
async fn get_serves_the_card_with_media_type_cors_and_caching() {
    let base = start(builder().with_server_card(card())).await;
    let resp = reqwest::Client::new()
        .get(format!("{base}{CARD_PATH}"))
        .header("Accept", "application/mcp-server-card+json")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(
        header(&resp, "content-type"),
        "application/mcp-server-card+json"
    );
    assert_eq!(header(&resp, "cache-control"), "public, max-age=3600");
    assert_card_cors(&resp);
    let etag = header(&resp, "etag");
    assert!(etag.starts_with('"') && etag.ends_with('"'), "{etag}");
    let body: Value = resp.json().await.unwrap();
    assert_eq!(
        body,
        json!({
            "$schema": "https://static.modelcontextprotocol.io/schemas/v1/server-card.schema.json",
            "name": "com.example/weather",
            "version": "1.2.0",
            "description": "Weather forecasts by city",
            "title": "Weather",
            "icons": [{ "src": "https://example.com/icon.png" }],
            "remotes": [{
                "type": "streamable-http",
                "url": "https://mcp.example.com/mcp",
                "supportedProtocolVersions": ["2026-07-28"]
            }]
        })
    );
}

#[tokio::test]
async fn card_fields_equal_server_discover() {
    let base = start(builder().with_server_card(card())).await;
    let card: Value = reqwest::get(format!("{base}{CARD_PATH}"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let d = discover(&base).await;
    // 2026-07-28 carries serverInfo in the result _meta.
    let info = &d["result"]["_meta"]["io.modelcontextprotocol/serverInfo"];
    assert_eq!(card["version"], info["version"], "{d}");
    assert_eq!(card["title"], info["title"], "{d}");
    assert_eq!(card["icons"], info["icons"], "{d}");
    assert_eq!(
        card["remotes"][0]["supportedProtocolVersions"], d["result"]["supportedVersions"],
        "{d}"
    );
}

#[tokio::test]
async fn matching_if_none_match_is_304() {
    let base = start(builder().with_server_card(card())).await;
    let client = reqwest::Client::new();
    let etag = header(
        &client
            .get(format!("{base}{CARD_PATH}"))
            .send()
            .await
            .unwrap(),
        "etag",
    );
    for tag in [
        etag.clone(),
        format!("W/{etag}"),
        format!("\"other\", {etag}"),
        "*".into(),
    ] {
        let resp = client
            .get(format!("{base}{CARD_PATH}"))
            .header("If-None-Match", &tag)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 304, "If-None-Match: {tag}");
        assert_eq!(header(&resp, "etag"), etag);
        assert_card_cors(&resp);
        assert!(resp.bytes().await.unwrap().is_empty());
    }
    let resp = client
        .get(format!("{base}{CARD_PATH}"))
        .header("If-None-Match", "\"stale\"")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn head_options_and_other_methods() {
    let base = start(builder().with_server_card(card())).await;
    let client = reqwest::Client::new();
    let url = format!("{base}{CARD_PATH}");

    let head = client.head(&url).send().await.unwrap();
    assert_eq!(head.status(), 200);
    assert_eq!(
        header(&head, "content-type"),
        "application/mcp-server-card+json"
    );
    assert!(head.bytes().await.unwrap().is_empty());

    let options = client
        .request(reqwest::Method::OPTIONS, &url)
        .header("Origin", "https://browser.example")
        .header("Access-Control-Request-Method", "GET")
        .send()
        .await
        .unwrap();
    assert_eq!(options.status(), 204);
    assert_card_cors(&options);

    let post = client.post(&url).body("{}").send().await.unwrap();
    assert_eq!(post.status(), 405);
    assert_eq!(header(&post, "allow"), "GET, HEAD, OPTIONS");
}

#[tokio::test]
async fn route_cors_survives_the_servers_own_cors() {
    // CORS is on by default on the HTTP server; the card's headers must win.
    let base = start(builder().cors(true).with_server_card(card())).await;
    let resp = reqwest::get(format!("{base}{CARD_PATH}")).await.unwrap();
    assert_card_cors(&resp);
}

#[tokio::test]
async fn a_default_build_serves_no_card() {
    let base = start(builder()).await;
    let resp = reqwest::get(format!("{base}{CARD_PATH}")).await.unwrap();
    assert_eq!(resp.status(), 404);
}

#[test]
fn a_card_contradicting_server_discover_fails_build() {
    let cases = [
        ("version", card().with_version("9.9.9")),
        ("title", card().with_title("Other")),
        (
            "supportedProtocolVersions",
            ServerCard::new("com.example/weather", "d").with_remote(
                Remote::new(RemoteType::StreamableHttp, "https://mcp.example.com/mcp")
                    .with_supported_protocol_versions(vec!["2025-11-25".into()]),
            ),
        ),
    ];
    for (field, card) in cases {
        let err = builder()
            .with_server_card(card)
            .build()
            .err()
            .unwrap_or_else(|| panic!("{field}: build accepted a contradicting card"))
            .to_string();
        assert!(err.contains(field), "{field}: {err}");
    }
    // Equal values are not a contradiction.
    assert!(
        builder()
            .with_server_card(card().with_version("1.2.0").with_title("Weather"))
            .build()
            .is_ok()
    );
}

#[test]
fn an_invalid_card_fails_build() {
    let err = builder()
        .with_server_card(ServerCard::new("no-slash", "d"))
        .build()
        .map(|_| ())
        .expect_err("build accepted an invalid card")
        .to_string();
    assert!(err.contains("name"), "{err}");
}

#[test]
fn a_description_comes_from_the_server_when_the_card_has_none() {
    // The card's description is required; an empty one is filled from the
    // server's, and a server description over 100 characters fails validation.
    assert!(
        builder()
            .description("Forecasts")
            .with_server_card(ServerCard::new("com.example/weather", ""))
            .build()
            .is_ok()
    );
    assert!(
        builder()
            .description("d".repeat(101))
            .with_server_card(ServerCard::new("com.example/weather", ""))
            .build()
            .is_err()
    );
}
