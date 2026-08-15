//! 2025-11-25 lane: the sessionless-ping bypass waives the SESSION
//! requirement only — pre-session auth middleware still verifies the
//! request ("MCP servers that implement authorization MUST verify all
//! inbound requests", security best practices §Session Hijacking
//! Mitigation).

use std::sync::Arc;

use async_trait::async_trait;
use turul_http_mcp_server::middleware::{
    DispatcherResult, McpMiddleware, MiddlewareError, RequestContext, SessionInjection,
};
use turul_mcp_server::McpServer;
use turul_mcp_session_storage::SessionView;

/// Pre-session middleware that challenges every request, like OAuth does.
struct ForceChallenge;

#[async_trait]
impl McpMiddleware for ForceChallenge {
    fn runs_before_session(&self) -> bool {
        true
    }
    async fn before_dispatch(
        &self,
        _ctx: &mut RequestContext<'_>,
        _session: Option<&dyn SessionView>,
        _injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        Err(MiddlewareError::http_challenge(
            401,
            "Bearer realm=\"mcp\", resource_metadata=\"https://rs.example.test/.well-known/oauth-protected-resource\"",
        ))
    }
    async fn after_dispatch(
        &self,
        _ctx: &RequestContext<'_>,
        _result: &mut DispatcherResult,
    ) -> Result<(), MiddlewareError> {
        Ok(())
    }
}

async fn start_server(with_auth: bool) -> String {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();

    let mut builder = McpServer::builder()
        .name("ping-auth-2025-test")
        .version("0.4.0")
        .bind_address(format!("127.0.0.1:{port}").parse().unwrap());
    if with_auth {
        builder = builder.middleware(Arc::new(ForceChallenge));
    }
    let server = builder.build().expect("build 2025 server");

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

async fn sessionless_ping(url: &str) -> reqwest::Response {
    reqwest::Client::new()
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2025-11-25")
        .json(&serde_json::json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" }))
        .send()
        .await
        .expect("ping POST")
}

/// With auth middleware: a sessionless ping is still verified — the bypass
/// must not route around the pre-session auth phase.
#[tokio::test]
async fn sessionless_ping_is_still_subject_to_auth() {
    let url = start_server(true).await;
    let resp = sessionless_ping(&url).await;
    assert_eq!(
        resp.status(),
        401,
        "the sessionless-ping bypass waives the session, not authorization"
    );
    assert!(
        resp.headers().contains_key("www-authenticate"),
        "the challenge must reach the wire"
    );
}

/// Without auth middleware: the bypass still admits pre-init pings (the
/// behavior `allow_unauthenticated_ping` documents).
#[tokio::test]
async fn sessionless_ping_without_auth_still_succeeds() {
    let url = start_server(false).await;
    let resp = sessionless_ping(&url).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.expect("json");
    assert!(
        body.get("result").is_some(),
        "sessionless ping must still answer: {body}"
    );
}
