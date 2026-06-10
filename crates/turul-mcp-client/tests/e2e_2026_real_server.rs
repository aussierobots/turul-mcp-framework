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

/// Echoes only after the client answers an elicitation (MRTR round trip).
#[derive(McpTool, Clone, Default)]
#[tool(name = "gated_echo", description = "Echo, but ask first", output = String)]
struct GatedEchoTool {}

impl GatedEchoTool {
    async fn execute(
        &self,
        session: Option<turul_mcp_server::SessionContext>,
    ) -> McpResult<String> {
        let session = session.ok_or_else(|| McpError::tool_execution("session required"))?;
        if let Some(responses) = session.input_responses() {
            let answer = responses
                .get("q1")
                .and_then(|r| match r {
                    turul_mcp_protocol::input_required::InputResponse::Elicit(e) => e
                        .content
                        .as_ref()
                        .and_then(|c| c.get("answer"))
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    _ => None,
                })
                .ok_or_else(|| McpError::tool_execution("q1 elicit response missing"))?;
            return Ok(format!("answered: {answer}"));
        }
        let schema = turul_mcp_protocol::elicitation::ElicitationSchema::new().with_property(
            "answer".to_string(),
            turul_mcp_protocol::elicitation::PrimitiveSchemaDefinition::string(),
        );
        let mut requests = turul_mcp_protocol::input_required::InputRequests::new();
        requests.insert(
            "q1".to_string(),
            turul_mcp_protocol::input_required::InputRequest::Elicit(
                turul_mcp_protocol::elicitation::ElicitRequest::new_form(
                    "What is the answer?",
                    schema,
                ),
            ),
        );
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: Some("st-9".to_string()),
        })
    }
}

#[tokio::test]
async fn mrtr_round_trip_through_the_bilingual_client() {
    // Server with the gated tool.
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let server = McpServer::builder()
        .name("e2e-2026-mrtr")
        .version("0.4.0")
        .tool(GatedEchoTool::default())
        .bind_address(format!("127.0.0.1:{port}").parse().unwrap())
        .build()
        .expect("build 2026 server");
    tokio::spawn(async move {
        server.run().await.ok();
    });
    let url = format!("http://127.0.0.1:{port}/mcp");
    let probe = reqwest::Client::new();
    for _ in 0..50 {
        if probe.get(&url).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    // Client that DECLARES the elicitation capability (servers reject MRTR
    // elicit requests against clients that did not declare it).
    let mut config = ClientConfig::default();
    config.declared_capabilities.elicitation = true;
    let transport = Box::new(HttpTransport::new(&url).unwrap());
    let client = McpClient::new(transport, config);
    client.connect().await.expect("connect");

    // Leg 1: the call surfaces InputRequired with the elicit request.
    let outcome = client.call_tool("gated_echo", json!({})).await;
    let (input_requests, request_state) = match outcome {
        Err(turul_mcp_client::McpClientError::InputRequired {
            input_requests,
            request_state,
        }) => (input_requests, request_state),
        other => panic!("expected InputRequired, got: {other:?}"),
    };
    let requests = input_requests.expect("inputRequests present");
    assert_eq!(
        requests["q1"]["method"], "elicitation/create",
        "the elicit request must surface to the application"
    );
    assert_eq!(request_state.as_deref(), Some("st-9"));

    // Leg 2: retry the original call with the gathered response + echoed state.
    let result = client
        .call_tool_with_input_responses(
            "gated_echo",
            json!({}),
            json!({ "q1": { "action": "accept", "content": { "answer": "42" } } }),
            request_state,
        )
        .await
        .expect("MRTR retry must complete");
    let text = serde_json::to_string(&result).unwrap_or_default();
    assert!(
        text.contains("answered: 42"),
        "the retry must complete the original call: {text}"
    );
}
