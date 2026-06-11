//! Middleware Rate Limiting Example
//!
//! Demonstrates rate limiting middleware on the 2026-07-28 stateless core:
//! 1. Tracks request counts per CLIENT IDENTITY (the `X-API-Key` header) —
//!    the 2026 lane has no sessions to key by, so the limiter runs in the
//!    pre-session phase and keys on a stateless identity. Requests without
//!    a key share one "anonymous" bucket.
//! 2. Returns MiddlewareError::RateLimitExceeded when the window limit hits
//! 3. Maps to -32003 JSON-RPC error with retryAfter data

use async_trait::async_trait;
use clap::Parser;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use turul_mcp_server::prelude::*;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "8671")]
    port: u16,
}

/// Rate limiting middleware with per-client-identity counters.
struct RateLimitMiddleware {
    max_requests: u32,
    window_secs: u64,
    /// Client identity (X-API-Key value, or "anonymous") -> (window start, count)
    counters: Mutex<HashMap<String, (std::time::Instant, u32)>>,
}

impl RateLimitMiddleware {
    fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            max_requests,
            window_secs,
            counters: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl McpMiddleware for RateLimitMiddleware {
    /// Pre-session: the request headers are available in `ctx.metadata()`
    /// and no session is required — exactly what a stateless limiter needs.
    fn runs_before_session(&self) -> bool {
        true
    }

    async fn before_dispatch(
        &self,
        ctx: &mut RequestContext<'_>,
        _session: Option<&dyn turul_mcp_session_storage::SessionView>,
        injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        // Stateless client identity: the X-API-Key header. Production
        // deployments key on whatever identity they trust (validated token
        // subject, client certificate, source address from the LB).
        let identity = ctx
            .metadata()
            .get("x-api-key")
            .and_then(|v| v.as_str())
            .unwrap_or("anonymous")
            .to_string();

        let window = std::time::Duration::from_secs(self.window_secs);
        let now = std::time::Instant::now();
        let mut counters = self.counters.lock().unwrap();
        let entry = counters.entry(identity.clone()).or_insert((now, 0));
        if now.duration_since(entry.0) >= window {
            *entry = (now, 0); // window elapsed — fresh bucket
        }

        if entry.1 >= self.max_requests {
            let retry_after = window
                .saturating_sub(now.duration_since(entry.0))
                .as_secs()
                .max(1);
            tracing::warn!(
                "Rate limit exceeded for {identity}: {} >= {} (retry in {retry_after}s)",
                entry.1,
                self.max_requests
            );
            return Err(MiddlewareError::RateLimitExceeded {
                message: format!(
                    "Rate limit exceeded: {} requests per {} seconds",
                    self.max_requests, self.window_secs
                ),
                retry_after: Some(retry_after),
            });
        }

        entry.1 += 1;
        injection.set_metadata("request_count", json!(entry.1));
        tracing::info!(
            "{identity} request count: {}/{}",
            entry.1,
            self.max_requests
        );

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

#[tokio::main]
async fn main() -> McpResult<()> {
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("middleware_rate_limit_server=info,turul_mcp_server=info")
        .init();

    tracing::info!(
        "Starting middleware-rate-limit-server example on port {}",
        args.port
    );
    tracing::info!("Rate limit: 5 requests per X-API-Key (or one shared anonymous bucket)");
    tracing::info!("After 5 requests, you'll receive error -32003 (RateLimitExceeded)");

    let bind_address: std::net::SocketAddr = format!("127.0.0.1:{}", args.port)
        .parse()
        .expect("Failed to parse bind address");

    let server = McpServer::builder()
        .name("middleware-rate-limit-server")
        .version("1.0.0")
        .title("Rate Limiting Middleware Example")
        .instructions(
            "Demonstrates stateless rate limiting. Max 5 requests per X-API-Key per window.",
        )
        // Register rate limiting middleware (5 requests per client identity)
        .middleware(Arc::new(RateLimitMiddleware::new(5, 60)))
        .bind_address(bind_address)
        .build()?;

    tracing::info!("Server listening on http://localhost:{}/mcp", args.port);
    tracing::info!("Try sending multiple requests to see rate limiting in action");

    server.run().await
}
