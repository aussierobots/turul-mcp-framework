//! End-to-end: the bilingual client against a REAL 2026-07-28 stateless server
//! (in-process `turul-mcp-server` on an ephemeral port — no mocks).
//!
//! This is the production path a 2026 deployment exercises: negotiation probes
//! `server/discover` with the full 2026 request-metadata headers
//! (`MCP-Protocol-Version: 2026-07-28` + `Mcp-Method`), the server's
//! §Server Validation enforces them, and every subsequent operation carries
//! `Mcp-Method`/`Mcp-Name` plus the per-request `_meta`.
#![cfg(feature = "client-bilingual")]

use serde_json::json;
use turul_mcp_client::config::ClientConfig;
use turul_mcp_client::transport::http::HttpTransport;
use turul_mcp_client::{McpClient, McpVersion};
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;

#[derive(McpTool, Clone, Default)]
#[tool(name = "echo", description = "Echo back the provided message", output = String)]
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

async fn start_2026_server() -> String {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();

    let server = McpServer::builder()
        .name("e2e-2026-real")
        .version("0.4.0")
        .tool(EchoTool::default())
        .bind_address(format!("127.0.0.1:{port}").parse().unwrap())
        .build()
        .expect("build 2026 server");

    tokio::spawn(async move {
        server.run().await.ok();
    });

    let url = format!("http://127.0.0.1:{port}/mcp");
    // 405 on GET = the accept loop is live (2026 endpoint is POST-only).
    let probe = reqwest::Client::new();
    for _ in 0..50 {
        if probe.get(&url).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    url
}

#[tokio::test]
async fn bilingual_client_negotiates_and_calls_tools_on_a_real_2026_server() {
    let url = start_2026_server().await;

    let transport = Box::new(HttpTransport::new(&url).unwrap());
    let client = McpClient::new(transport, ClientConfig::default());
    client
        .connect()
        .await
        .expect("negotiation against the real 2026 server must succeed");

    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2026_07_28),
        "the real 2026 server must be detected via server/discover"
    );

    // tools/list — requires MCP-Protocol-Version + Mcp-Method on the wire.
    let tools = client.list_tools().await.expect("list_tools");
    assert!(
        tools.iter().any(|t| t.name == "echo"),
        "echo tool must be advertised"
    );

    // tools/call — additionally requires Mcp-Name matching params.name.
    let result = client
        .call_tool("echo", json!({ "message": "round-trip" }))
        .await
        .expect("call_tool");
    let text = serde_json::to_string(&result).unwrap_or_default();
    assert!(
        text.contains("Echo: round-trip"),
        "tool result must round-trip through the real server: {text}"
    );
}
