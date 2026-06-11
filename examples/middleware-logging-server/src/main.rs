//! Middleware Logging Example
//!
//! Demonstrates request timing and tracing middleware that:
//! 1. Captures the request start time in before_dispatch (stored on the
//!    request context's metadata, which the dispatcher threads through to
//!    after_dispatch)
//! 2. Logs the measured request duration in after_dispatch

use async_trait::async_trait;
use clap::Parser;
use serde_json::json;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use turul_mcp_server::prelude::*;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "8670")]
    port: u16,
}

/// Logging middleware that tracks request timing
struct TimingMiddleware;

#[async_trait]
impl McpMiddleware for TimingMiddleware {
    async fn before_dispatch(
        &self,
        ctx: &mut RequestContext<'_>,
        _session: Option<&dyn turul_mcp_session_storage::SessionView>,
        _injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        let start_us = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;
        ctx.add_metadata("timing_start_us", json!(start_us));

        tracing::info!("→ {} starting", ctx.method());
        Ok(())
    }

    async fn after_dispatch(
        &self,
        ctx: &RequestContext<'_>,
        result: &mut DispatcherResult,
    ) -> Result<(), MiddlewareError> {
        let now_us = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;
        let elapsed_ms = ctx
            .metadata()
            .get("timing_start_us")
            .and_then(|v| v.as_u64())
            .map(|start_us| (now_us.saturating_sub(start_us)) as f64 / 1000.0);

        match elapsed_ms {
            Some(ms) => tracing::info!(
                "← {} completed in {ms:.2}ms ({})",
                ctx.method(),
                if result.is_success() { "ok" } else { "error" }
            ),
            None => tracing::info!("← {} completed", ctx.method()),
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> McpResult<()> {
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("middleware_logging_server=info,turul_mcp_server=info")
        .init();

    tracing::info!(
        "Starting middleware-logging-server example on port {}",
        args.port
    );
    tracing::info!("All requests will be logged with timing information");

    let bind_address: std::net::SocketAddr = format!("127.0.0.1:{}", args.port)
        .parse()
        .expect("Failed to parse bind address");

    let server = McpServer::builder()
        .name("middleware-logging-server")
        .version("1.0.0")
        .title("Request Timing Middleware Example")
        .instructions("Demonstrates request timing and tracing middleware. Every request is logged with timing info.")
        // Register timing middleware - this is the key demonstration
        .middleware(Arc::new(TimingMiddleware))
        .bind_address(bind_address)
        .build()?;

    tracing::info!("Server listening on http://localhost:{}/mcp", args.port);
    tracing::info!(
        "Try: curl -X POST http://localhost:{}/mcp -H 'Content-Type: application/json' -H 'MCP-Protocol-Version: 2026-07-28' -H 'Mcp-Method: server/discover' -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"server/discover\",\"params\":{{\"_meta\":{{\"io.modelcontextprotocol/protocolVersion\":\"2026-07-28\",\"io.modelcontextprotocol/clientInfo\":{{\"name\":\"curl\",\"version\":\"1.0\"}},\"io.modelcontextprotocol/clientCapabilities\":{{}}}}}}}}'",
        args.port
    );

    server.run().await
}
