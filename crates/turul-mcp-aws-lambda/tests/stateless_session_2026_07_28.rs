//! 2026-07-28 stateless-lane session construction over the buffered Lambda transport.
//!
//! The stateless core removes the client-visible `Mcp-Session-Id`, but every
//! dispatched request still carries an internally-minted per-request session.
//! Middleware state written via `SessionInjection::set_state` must therefore be
//! readable from the tool's `SessionContext` — over the Lambda transport exactly
//! as over the local `turul-http-mcp-server` transport.
//!
//! Exercises the production path: `LambdaMcpServerBuilder` → `server.handler()`
//! → `LambdaMcpHandler::handle()` (buffered, non-streaming Lambda runtime).
#![cfg(feature = "protocol-2026-07-28")]

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use turul_http_mcp_server::middleware::{
    DispatcherResult, McpMiddleware, MiddlewareError, RequestContext, SessionInjection,
};
use turul_mcp_aws_lambda::{LambdaMcpHandler, LambdaMcpServerBuilder};
use turul_mcp_derive::McpTool;
use turul_mcp_protocol::McpError;
use turul_mcp_server::{McpResult, SessionContext};
use turul_mcp_session_storage::SessionView;

/// Simulates an authenticating middleware (e.g. API Gateway authorizer bridge)
/// that stamps the caller's identity into per-request session state.
struct AccountInjectingMiddleware;

#[async_trait]
impl McpMiddleware for AccountInjectingMiddleware {
    async fn before_dispatch(
        &self,
        _ctx: &mut RequestContext<'_>,
        _session: Option<&dyn SessionView>,
        injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        injection.set_state("account_id", json!("acct-42"));
        Ok(())
    }

    async fn after_dispatch(
        &self,
        _ctx: &RequestContext<'_>,
        _result: &mut DispatcherResult,
    ) -> Result<(), MiddlewareError> {
        Ok(())
    }
}

/// Tool that requires a session and echoes the middleware-injected account id.
#[derive(McpTool, Clone, Default)]
#[tool(
    name = "whoami",
    description = "Return the middleware-injected account id",
    output = String
)]
struct WhoamiTool {}

impl WhoamiTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        let session = session.ok_or_else(|| McpError::InvalidRequest {
            message: "session required".to_string(),
        })?;
        session
            .get_typed_state::<String>("account_id")
            .await
            .ok_or_else(|| McpError::InvalidRequest {
                message: "account_id missing from session".to_string(),
            })
    }
}

fn request_meta() -> serde_json::Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "test-client", "version": "1.0.0" },
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

async fn build_handler() -> LambdaMcpHandler {
    let server = LambdaMcpServerBuilder::new()
        .name("stateless-session-test")
        .version("1.0.0")
        .tool(WhoamiTool::default())
        .middleware(Arc::new(AccountInjectingMiddleware))
        .build()
        .await
        .expect("build lambda server");
    server.handler().await.expect("create lambda handler")
}

fn whoami_request() -> lambda_http::Request {
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "whoami",
            "arguments": {},
            "_meta": request_meta()
        }
    });
    http::Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "whoami")
        .body(lambda_http::Body::from(
            serde_json::to_string(&body).unwrap(),
        ))
        .unwrap()
}

fn response_json(response: &lambda_http::Response<lambda_http::Body>) -> serde_json::Value {
    let bytes = match response.body() {
        lambda_http::Body::Text(s) => s.as_bytes().to_vec(),
        lambda_http::Body::Binary(b) => b.clone(),
        _ => Vec::new(),
    };
    serde_json::from_slice(&bytes).expect("response body must be JSON")
}

/// A middleware-authenticated tools/call over the buffered Lambda transport
/// receives a per-request session carrying the injected state.
#[tokio::test]
async fn tool_receives_session_with_middleware_state_over_lambda_handle() {
    let handler = build_handler().await;
    let response = handler.handle(whoami_request()).await.expect("handle");

    assert_eq!(response.status(), 200, "tools/call must succeed");
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok()),
        Some("application/json"),
        "JSON framing expected for Accept: application/json"
    );

    let body = response_json(&response);
    assert!(
        body.get("error").is_none(),
        "expected success result, got error: {body}"
    );
    let text = body["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("missing result.content[0].text in: {body}"));
    assert!(
        text.contains("acct-42"),
        "tool must read middleware-injected account_id, got: {text}"
    );
}

/// Middleware that rejects every request, simulating failed authentication.
struct BlockingMiddleware;

#[async_trait]
impl McpMiddleware for BlockingMiddleware {
    async fn before_dispatch(
        &self,
        _ctx: &mut RequestContext<'_>,
        _session: Option<&dyn SessionView>,
        _injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        Err(MiddlewareError::Unauthenticated(
            "Lambda auth required".to_string(),
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

/// A middleware rejection short-circuits dispatch and surfaces as a JSON-RPC
/// error (-32001 Unauthenticated) over the buffered Lambda transport.
#[tokio::test]
async fn middleware_error_short_circuits_over_lambda_handle() {
    let server = LambdaMcpServerBuilder::new()
        .name("stateless-session-test")
        .version("1.0.0")
        .tool(WhoamiTool::default())
        .middleware(Arc::new(BlockingMiddleware))
        .build()
        .await
        .expect("build lambda server");
    let handler = server.handler().await.expect("create lambda handler");

    let response = handler.handle(whoami_request()).await.expect("handle");
    assert_eq!(response.status(), 200, "JSON-RPC errors ride HTTP 200");

    let body = response_json(&response);
    assert_eq!(
        body["error"]["code"].as_i64(),
        Some(-32001),
        "expected Unauthenticated, got: {body}"
    );
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("Lambda auth required"),
        "middleware message must surface, got: {body}"
    );
}

/// The buffered Lambda lane enforces 2026-07-28 Server Validation: a request
/// missing the MCP-Protocol-Version header is rejected with HTTP 400 and
/// JSON-RPC -32020 (HeaderMismatch), not silently dispatched.
#[tokio::test]
async fn missing_protocol_version_header_rejected_over_lambda_handle() {
    let handler = build_handler().await;

    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "whoami",
            "arguments": {},
            "_meta": request_meta()
        }
    });
    let request = http::Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "whoami")
        .body(lambda_http::Body::from(
            serde_json::to_string(&body).unwrap(),
        ))
        .unwrap();

    let response = handler.handle(request).await.expect("handle");
    assert_eq!(response.status(), 400, "header validation must reject");

    let body = response_json(&response);
    assert_eq!(
        body["error"]["code"].as_i64(),
        Some(turul_mcp_protocol::headers::ERROR_CODE_HEADER_MISMATCH),
        "expected -32020 HeaderMismatch, got: {body}"
    );
}
