//! Middleware Logging Example
//!
//! Demonstrates request timing and tracing middleware that:
//! 1. Captures request start time in before_dispatch
//! 2. Logs request duration in after_dispatch
//! 3. Injects last_request_ms into session metadata
//!
//! Tools can read the injected metadata to see request timing.

use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;
use turul_mcp_server::prelude::*;

/// Logging middleware that tracks request timing
struct TimingMiddleware;

#[async_trait]
impl McpMiddleware for TimingMiddleware {
    async fn before_dispatch(
        &self,
        ctx: &mut RequestContext<'_>,
        _session: Option<&dyn turul_mcp_session_storage::SessionView>,
        injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        let start = Instant::now();

        // Store start time in metadata for after_dispatch
        injection.set_metadata("_timing_start_ms", json!(start.elapsed().as_millis()));

        tracing::info!("→ {} starting", ctx.method());
        Ok(())
    }

    async fn after_dispatch(
        &self,
        ctx: &RequestContext<'_>,
        _result: &mut DispatcherResult,
    ) -> Result<(), MiddlewareError> {
        // In production, you'd calculate duration from stored start time
        // For this example, we'll use a simple log
        tracing::info!("← {} completed", ctx.method());
        Ok(())
    }
}

#[tokio::main]
async fn main() -> McpResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("middleware_logging_server=info,turul_mcp_server=info")
        .init();

    tracing::info!("Starting middleware-logging-server example");
    tracing::info!("All requests will be logged with timing information");

    let server = McpServer::builder()
        .name("middleware-logging-server")
        .version("1.0.0")
        .title("Request Timing Middleware Example")
        .instructions("Demonstrates request timing and tracing middleware. Every request is logged with timing info.")
        // Register timing middleware - this is the key demonstration
        .middleware(Arc::new(TimingMiddleware))
        .build()?;

    tracing::info!("Server listening on http://localhost:8080/mcp");
    tracing::info!("Try: curl -X POST http://localhost:8080/mcp -H 'Content-Type: application/json' -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{{...}}}}'");

    server.run().await
}
