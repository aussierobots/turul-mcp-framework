//! # Notification Server (2026-07-28)
//!
//! Demonstrates BOTH server-initiated notification surfaces of the 2026
//! stateless core:
//!
//! 1. **Subscription notifications** — list-changed and per-URI
//!    `resources/updated` events delivered on long-lived
//!    `subscriptions/listen` POST streams. The `trigger_changes` tool
//!    broadcasts a batch so an open listen stream visibly receives its
//!    filtered subset, each stamped with the stream's `subscriptionId`.
//! 2. **Request-scoped notifications** — `notifications/progress` (opt-in
//!    via `_meta.progressToken`) and `notifications/message` (opt-in via
//!    the `_meta` `logLevel`) riding the originating POST's own SSE
//!    response, emitted by the `long_job` tool.
//!
//! There is no GET SSE stream and no session on this lane: the endpoint is
//! POST-only, and stream state is request-scoped (drop a listen stream and
//! re-issue it to "reconnect").

use std::collections::HashMap;

use turul_http_mcp_server::notification_bridge::SharedNotificationBroadcaster;
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;
use turul_rpc::JsonRpcNotification;

/// Broadcasts one notification of each subscription flavor — open a
/// `subscriptions/listen` stream first, then call this.
#[derive(McpTool, Clone, Default)]
#[tool(
    name = "trigger_changes",
    description = "Broadcast list-changed + resources/updated notifications to open listen streams",
    output = String
)]
struct TriggerChangesTool {}

impl TriggerChangesTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        let session = session.ok_or_else(|| McpError::tool_execution("context required"))?;
        let any = session
            .broadcaster
            .as_ref()
            .ok_or_else(|| McpError::tool_execution("broadcaster required"))?;
        let broadcaster = any
            .downcast_ref::<SharedNotificationBroadcaster>()
            .ok_or_else(|| McpError::tool_execution("broadcaster type mismatch"))?
            .clone();

        for method in [
            "notifications/resources/list_changed",
            "notifications/prompts/list_changed",
            "notifications/tools/list_changed",
        ] {
            let _ = broadcaster
                .broadcast_to_all_sessions(JsonRpcNotification::new_no_params(method.to_string()))
                .await;
        }
        let mut updated = HashMap::new();
        updated.insert("uri".to_string(), serde_json::json!("file:///watched.txt"));
        let _ = broadcaster
            .broadcast_to_all_sessions(JsonRpcNotification::new_with_object_params(
                "notifications/resources/updated".to_string(),
                updated,
            ))
            .await;

        Ok("broadcast 3 list-changed + 1 resources/updated — check your listen stream".to_string())
    }
}

/// Emits request-scoped progress and log notifications while it "works".
/// Progress requires `_meta.progressToken`; log lines require the `_meta`
/// `logLevel` opt-in — without them the server stays silent (per spec).
#[derive(McpTool, Clone, Default)]
#[tool(
    name = "long_job",
    description = "Simulated job emitting request-scoped progress + log notifications",
    output = String
)]
struct LongJobTool {}

impl LongJobTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        let session = session.ok_or_else(|| McpError::tool_execution("context required"))?;
        for step in 1..=3u8 {
            session
                .notify_request_progress_with_message(
                    f64::from(step) / 3.0,
                    Some(1.0),
                    format!("step {step}/3"),
                )
                .await;
            #[allow(deprecated)] // notifications/message is SEP-2577-deprecated but normative
            session
                .notify_log(
                    turul_mcp_protocol::logging::LoggingLevel::Info,
                    serde_json::json!(format!("long_job step {step} complete")),
                    Some("long_job".to_string()),
                    None,
                )
                .await;
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        }
        Ok("job done — 3 progress + 3 log notifications rode this request's stream".to_string())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let server = McpServer::builder()
        .name("notification-server")
        .version("0.4.0")
        .title("2026 Notification Surfaces Demo")
        .instructions(
            "Open a subscriptions/listen stream, then call trigger_changes to see \
             subscription notifications; call long_job with a progressToken and \
             logLevel in _meta to see request-scoped notifications.",
        )
        .tool(TriggerChangesTool::default())
        .tool(LongJobTool::default())
        .with_resources()
        .with_prompts()
        .bind_address("127.0.0.1:8005".parse()?)
        .build()?;

    tracing::info!("🚀 Notification server at http://127.0.0.1:8005/mcp (POST-only)");
    tracing::info!("1) Open a listen stream (long-lived POST SSE):");
    tracing::info!(
        "   curl -N -X POST http://127.0.0.1:8005/mcp -H 'Content-Type: application/json' \\"
    );
    tracing::info!("     -H 'Accept: text/event-stream' -H 'MCP-Protocol-Version: 2026-07-28' \\");
    tracing::info!("     -H 'Mcp-Method: subscriptions/listen' \\");
    tracing::info!(
        "     -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"subscriptions/listen\",\"params\":{{\"notifications\":{{\"resourcesListChanged\":true,\"toolsListChanged\":true,\"resourceSubscriptions\":[\"file:///watched.txt\"]}},\"_meta\":{{\"io.modelcontextprotocol/protocolVersion\":\"2026-07-28\",\"io.modelcontextprotocol/clientInfo\":{{\"name\":\"curl\",\"version\":\"1.0\"}},\"io.modelcontextprotocol/clientCapabilities\":{{}}}}}}}}'"
    );
    tracing::info!("2) In another terminal, call trigger_changes (tools/call) — the listen");
    tracing::info!("   stream receives its filtered subset, stamped with subscriptionId.");
    tracing::info!("3) Call long_job with _meta.progressToken + logLevel and");
    tracing::info!("   Accept: application/json, text/event-stream — progress/log ride the");
    tracing::info!("   request's own response stream before the final result.");

    server.run().await?;
    Ok(())
}
