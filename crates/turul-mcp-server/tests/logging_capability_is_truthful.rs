//! A server that registers no `McpLogger` still serves logging, so advertising
//! `capabilities.logging` is truthful.
//!
//! The capability was reported as a truthfulness violation on the grounds that
//! `builder.rs` set it unconditionally while computing an unread
//! `has_logging = !loggers.is_empty()`. The unread computation was real and is
//! gone; the violation was not. Nothing about serving logging consults
//! `loggers`: `logging/setLevel` is registered in the default handler set and
//! stores the threshold on the session, and `SessionContext::notify_log` is
//! available to any tool. Gating the advertisement on a registered logger would
//! have made every server under-advertise a capability it honours — and no
//! `impl McpLogger` exists outside the derive macro's own tests, so the gate
//! would have silenced the capability repo-wide.
//!
//! This asserts the pairing directly: the advertisement and the behaviour it
//! promises, from one server built the way the report described — one tool, no
//! logger. The 2026 lane's half is `log_gating_2026.rs`, which drives
//! `notifications/message` delivery against an equally logger-less server.
#![cfg(feature = "protocol-2025-11-25")]

mod common;

use turul_mcp_derive::mcp_tool;
use turul_mcp_server::prelude::*;

#[mcp_tool(name = "echo", description = "Echo a string back")]
async fn echo(text: String) -> McpResult<String> {
    Ok(text)
}

struct Fixture {
    url: String,
    client: reqwest::Client,
    session: String,
}

/// Builds the server the bug report described — one tool, `.logger()` never
/// called — and completes the 2025-11-25 handshake against it.
async fn handshake() -> (Fixture, serde_json::Value) {
    let reserved = common::reserve_port().await;
    let port = reserved.port;

    let server = McpServer::builder()
        .name("logging-truthfulness")
        .version("0.4.0")
        .tool_fn(echo)
        .bind_address(format!("127.0.0.1:{port}").parse().unwrap())
        .build()
        .expect("build 2025-11-25 server");

    tokio::spawn(async move {
        server.run().await.ok();
    });

    let url = format!("http://127.0.0.1:{port}/mcp");
    let client = reqwest::Client::new();
    for _ in 0..100 {
        if client.get(&url).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    drop(reserved);

    let init = client
        .post(&url)
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": { "name": "truthfulness-probe", "version": "0.4.0" }
            }
        }))
        .send()
        .await
        .expect("initialize POST");

    let session = init
        .headers()
        .get("mcp-session-id")
        .expect("2025-11-25 initialize must mint a session id")
        .to_str()
        .expect("session id is ascii")
        .to_string();
    let body: serde_json::Value = init.json().await.expect("initialize body");

    let accepted = client
        .post(&url)
        .header("Accept", "application/json")
        .header("Mcp-Session-Id", &session)
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "method": "notifications/initialized"
        }))
        .send()
        .await
        .expect("initialized POST");
    assert_eq!(accepted.status(), 202, "handshake must complete");

    (
        Fixture {
            url,
            client,
            session,
        },
        body,
    )
}

#[tokio::test]
async fn a_server_with_no_logger_advertises_the_logging_capability() {
    let (_f, body) = handshake().await;
    assert!(
        body["result"]["capabilities"]["logging"].is_object(),
        "logging is served unconditionally, so it must be advertised: {body}"
    );
}

/// The other half — without this, the assertion above is only a claim that the
/// server repeats its own configuration back.
#[tokio::test]
async fn and_honours_logging_set_level() {
    let (f, _) = handshake().await;

    let body: serde_json::Value = f
        .client
        .post(&f.url)
        .header("Accept", "application/json")
        .header("Mcp-Session-Id", &f.session)
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "logging/setLevel",
            "params": { "level": "debug" }
        }))
        .send()
        .await
        .expect("setLevel POST")
        .json()
        .await
        .expect("setLevel body");

    assert!(
        body.get("error").is_none(),
        "a logger-less server must still honour the capability it advertises: {body}"
    );
    assert!(
        body.get("result").is_some(),
        "logging/setLevel returns an empty result on success: {body}"
    );
}
