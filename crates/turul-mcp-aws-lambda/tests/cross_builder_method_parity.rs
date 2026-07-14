//! Cross-builder method-registration parity.
//!
//! The local `McpServer` builder (`turul-mcp-server`) and the
//! `LambdaMcpServerBuilder` are independent code paths that each register the
//! JSON-RPC method handlers for the active protocol lane. They MUST register
//! the same method set — otherwise one transport silently serves a method the
//! other 404s (exactly the `server/discover`-missing-on-Lambda bug fixed
//! 2026-07-13). These tests build both through their production paths, read the
//! dispatcher's registered method set from each, and assert they are identical
//! per lane. Adding a method to one builder but not the other fails here.

#![cfg(any(feature = "protocol-2026-07-28", feature = "protocol-2025-11-25"))]

use std::collections::BTreeSet;
use std::sync::Arc;

use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
use turul_mcp_derive::McpTool;
use turul_mcp_server::{McpResult, McpServer, SessionContext};
use turul_mcp_session_storage::InMemorySessionStorage;

#[derive(McpTool, Clone, Default)]
#[tool(name = "probe", description = "cross-builder parity probe", output = String)]
struct ProbeTool {}

impl ProbeTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok("ok".to_string())
    }
}

fn local_registered_methods() -> BTreeSet<String> {
    let server = McpServer::builder()
        .name("parity")
        .version("1.0.0")
        .tool(ProbeTool::default())
        .build()
        .expect("build local McpServer");
    server.registered_methods().into_iter().collect()
}

async fn lambda_registered_methods() -> BTreeSet<String> {
    let server = LambdaMcpServerBuilder::new()
        .name("parity")
        .version("1.0.0")
        .tool(ProbeTool::default())
        .storage(Arc::new(InMemorySessionStorage::new()))
        .sse(false)
        .build()
        .await
        .expect("build LambdaMcpServer");
    server
        .handler()
        .await
        .expect("lambda handler")
        .registered_methods()
        .into_iter()
        .collect()
}

/// The two builders must register an identical method set for the active lane.
#[tokio::test]
async fn lambda_and_local_register_identical_method_sets() {
    let local = local_registered_methods();
    let lambda = lambda_registered_methods().await;

    let only_local: Vec<&String> = local.difference(&lambda).collect();
    let only_lambda: Vec<&String> = lambda.difference(&local).collect();

    assert!(
        only_local.is_empty() && only_lambda.is_empty(),
        "Lambda vs local builder method-set divergence for this lane:\n  \
         only in local McpServer: {only_local:?}\n  \
         only in Lambda:          {only_lambda:?}\n  \
         (a method registered by one builder but not the other — the two must stay in parity)"
    );

    // Both must serve the lane's core discovery entry point.
    #[cfg(feature = "protocol-2026-07-28")]
    assert!(
        local.contains("server/discover"),
        "2026 lane must register server/discover on both builders"
    );
    #[cfg(feature = "protocol-2025-11-25")]
    assert!(
        local.contains("initialize"),
        "2025-11-25 lane must register initialize on both builders"
    );
}
