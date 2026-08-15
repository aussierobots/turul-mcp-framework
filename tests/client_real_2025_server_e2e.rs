//! End-to-end: `turul-mcp-client` against a REAL 2025-11-25 turul server.
//!
//! # Why this file exists
//!
//! The 2026-07-28 half of ADR-030's bilingual contract is well covered against a
//! real server (`crates/turul-mcp-client/tests/e2e_2026_real_server.rs`). The
//! 2025-11-25 half was not, and the shape of the gap mattered:
//! `bilingual_negotiation.rs` proves the *downgrade decision* correctly, but
//! every 2025 case there answers from a `wiremock::MockServer` — a hand-written
//! stub of what we believe a 2025 server does.
//!
//! A mock cannot disagree with the client, because the same author wrote both.
//! So if `turul-mcp-server`'s 2025 lane and `turul-mcp-client`'s 2025 lane
//! drifted apart, nothing in the repo would have noticed. `ci-gates.sh` only
//! *built* `client-initialise-server` and never pointed the client at it.
//!
//! This crate is the right home: its `turul-mcp-server` dependency is already
//! pinned to `protocol-2025-11-25`, while `turul-mcp-client` arrives with
//! default features (bilingual). One process, both real, no stubs. The client
//! crate's own test targets cannot do this — its dev-dependency on
//! `turul-mcp-server` takes default features, which is the 2026 lane.
//!
//! # What the journey proves
//!
//! Everything here is a 2025-11-25 mechanism that 2026-07-28 removed, so this
//! is also the regression guard for the opt-in lane continuing to work at all:
//!
//!   1. `server/discover` is answered `-32601` by a 2025 server, and the
//!      bilingual client reads that — and only that — as "fall back".
//!   2. The `initialize` handshake completes and the server mints an
//!      `Mcp-Session-Id`.
//!   3. That session id is reused on subsequent requests rather than a fresh
//!      one being minted per call.
//!   4. `tools/list` and `tools/call` work over the established session.

use turul_mcp_client::config::ClientConfig;
use turul_mcp_client::transport::http::HttpTransport;
use turul_mcp_client::{McpClient, McpVersion};
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;

#[derive(McpTool, Clone, Default)]
#[tool(
    name = "echo",
    description = "Echo back the provided message",
    output = String
)]
struct EchoTool {
    #[param(description = "Message to echo back")]
    message: String,
}

impl EchoTool {
    async fn execute(
        &self,
        _session: Option<turul_mcp_server::SessionContext>,
    ) -> McpResult<String> {
        Ok(format!("Echo: {}", self.message))
    }
}

/// Spawn an in-process 2025-11-25 server on an ephemeral port.
///
/// Ephemeral rather than fixed: the suite runs test binaries in parallel, and a
/// hardcoded port makes failures depend on execution order (the defect
/// `2026 test servers race on port binding under load` already cost this repo
/// once).
async fn start_2025_server() -> String {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();

    let server = McpServer::builder()
        .name("client-real-2025-e2e")
        .version("0.4.0")
        .tool(EchoTool::default())
        .bind_address(format!("127.0.0.1:{port}").parse().unwrap())
        .build()
        .expect("build 2025-11-25 server");

    tokio::spawn(async move {
        server.run().await.ok();
    });

    let url = format!("http://127.0.0.1:{port}/mcp");
    let probe = reqwest::Client::new();
    for _ in 0..100 {
        if probe.get(&url).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    url
}

async fn connect(url: &str) -> McpClient {
    let transport = Box::new(HttpTransport::new(url).unwrap());
    let client = McpClient::new(transport, ClientConfig::default());
    client.connect().await.expect("connect to the 2025 server");
    client
}

#[tokio::test]
async fn bilingual_client_falls_back_to_2025_against_a_real_server() {
    let url = start_2025_server().await;
    let client = connect(&url).await;

    // The whole point: a REAL 2025 server answered the discover probe, and the
    // client locked the legacy spec off that answer rather than off a stub.
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2025_11_25),
        "a server with no server/discover must be driven as 2025-11-25"
    );

    let status = client.connection_status().await;
    assert_eq!(
        status.protocol_version.as_deref(),
        Some("2025-11-25"),
        "the negotiated version the handshake reported must be the legacy one"
    );

    // Sessions are the mechanism 2026-07-28 deleted. If the server stopped
    // minting one, every stateful 2025 behaviour downstream would silently
    // degrade to per-request state, so assert it directly rather than inferring
    // it from a later call happening to work.
    assert!(
        status.session_id.is_some(),
        "a 2025-11-25 server must mint an Mcp-Session-Id during initialize; got {:?}",
        status.session_id
    );

    client.disconnect().await.ok();
}

#[tokio::test]
async fn the_2025_session_id_is_reused_across_requests() {
    let url = start_2025_server().await;
    let client = connect(&url).await;

    let first = client
        .connection_status()
        .await
        .session_id
        .expect("session id after initialize");

    // Drive real traffic between the two observations. A session id that
    // survives an idle client proves nothing; one that survives requests is
    // what "stateful" means on this lane.
    client
        .list_tools()
        .await
        .expect("tools/list over the session");
    client
        .call_tool("echo", serde_json::json!({"message": "session reuse"}))
        .await
        .expect("tools/call over the session");

    let second = client
        .connection_status()
        .await
        .session_id
        .expect("session id after traffic");

    assert_eq!(
        first, second,
        "the client must reuse the minted session id, not re-handshake per request"
    );

    client.disconnect().await.ok();
}

#[tokio::test]
async fn tools_round_trip_over_the_2025_handshake() {
    let url = start_2025_server().await;
    let client = connect(&url).await;

    let tools = client.list_tools().await.expect("tools/list");
    assert!(
        tools.iter().any(|t| t.name == "echo"),
        "the server registered `echo`; tools/list returned {:?}",
        tools.iter().map(|t| &t.name).collect::<Vec<_>>()
    );

    let result = client
        .call_tool("echo", serde_json::json!({"message": "hello 2025"}))
        .await
        .expect("tools/call");

    // Assert on the returned value, not merely that the call returned Ok. A
    // tool that answered with the wrong payload would otherwise pass.
    let rendered = serde_json::to_string(&result).expect("serialize CallToolResult");
    assert!(
        rendered.contains("Echo: hello 2025"),
        "tools/call must carry the tool's output; got {rendered}"
    );

    client.disconnect().await.ok();
}
